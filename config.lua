-- TODO: move this block into `require "nnn"`, maybe
local nnn = {
	absent = {},  -- special tag value for marking absent files

	-- below values should be filled by user

	prereqs = {},
	wanted = {},
	-- ospath should be function translating a git relative path to
	-- absolute path in user's OS; currently, example handler for Windows
	-- OS is provided
	ospath = function(path)
		if #path < 3 then
			error(('path too short for Windows, must be "<DISK>/<RELPATH>", got: %q'):format(path))
		elseif path:sub(2,2) ~= '/' then
			error(('path must specify disk for Windows, must be "<DISK>/<RELPATH>", got: %q'):format(path))
		end
		return path:sub(1,1) .. ":\\" .. path:sub(3):gsub("/", "\\")
	end,
	-- oscopy should invoke operating system's copy function to copy file
	-- from p1 to p2 (OS paths); currently, example handler for Windows OS
	-- is provided
	oscopy = function(p1, p2)
		-- TODO: use rsync if possible
		local cmd = "copy /b /y " .. p1 .. " " .. p2
		io.stdout:write('# ' .. cmd .. '\n')
		assert(os.execute(cmd))
	end,
	-- osmkdirp should create all parent directories leading to the
	-- specified OS path. This should be equivalent to Linux command:
	-- `mkdir -p "$(dirname "$path")"`
	osmkdirp = function(path)
		local iter = path:gmatch "([^\\]+)\\"
		local parent = iter()  -- "C:" or similar
		for d in iter do
			parent = parent .. '\\' .. d
			os.execute("mkdir " .. parent)
		end
	end,
}

local bin = "c/bin"

local oneliners = {
	gd = "git diff";
	gds = "git diff --staged";
	gs = "git status";
	gf = "git fetch --all";
	glg = "git log --graph \"--pretty=format:%%Cred%%h%%Creset -%%C(yellow)%%d%%Creset %%s %%Cgreen(%%cr %%cd) %%C(cyan)%%an%%Creset\" \"--date=format:'%%d.%%m\" --abbrev-commit";
	gl = "glg --all";
}

-- Render oneliners to full files
for name, text in pairs(oneliners) do
	-- if os == "windows" then
		nnn.wanted[bin .. "/" .. name .. ".bat"] = "@" .. text .. " %*"
	-- end
end

----------------------------------------------------
-- TODO: move the stuff below to proper "nnn.lua" --
----------------------------------------------------

---------------------------------------
-- more or less generic helper funcs --
---------------------------------------

-- die prints err on stderr and exits the app
local function die(err)
	io.stderr:write('error: ', err, '\n')
	os.exit(1)
end
-- or_die returns all args if v is not nil; otherwise, prints err and exits the app
local function or_die(v, err, ...)
	if not v then
		die(err)
	end
	return v, err, ...
end

local function has_prefix(s, prefix)
	return #s >= #prefix and s:sub(1,#prefix) == prefix
end

local function exists(ospath)
	local fh, err = io.open(ospath, 'r')
	if fh then fh:close() end
	return not not fh
end

local function assert_gitpath(s)
	if #s == 0 then
		error("empty path")
	end
	local function assert_no(pattern, msg)
		local from, to = s:find(pattern)
		if from then
			error(("%s: found %q at offset %d in path: %q"):format(msg, s:sub(from,to), from, s))
		end
	end
	-- FIXME: also sanitize other Windows-specific stuff, like 'nul' or 'con' in filenames
	-- FIXME: properly handle case-insensitive filesystems (Windows, Mac?)
	assert_no("[^%w_/.-]", "TODO: unsupported character")
	-- FIXME: actually *require* absolute paths, then de-absolutize them
	assert_no("^/", "absolute path")
	assert_no("/%.%./", "relative path")
	assert_no("^%.%./", "relative path")
	assert_no("/%.%.$", "relative path")
	assert_no("^%.%.$", "relative path")
	assert_no("/%./", "denormalized path")
	assert_no("^%./", "denormalized path")
	assert_no("/%.$", "denormalized path")
	assert_no("^%.$", "denormalized path")
	assert_no("/$", "trailing slash")
	assert_no("//", "duplicate slash")
end


-------------------------------
-- app-specific helper funcs --
-------------------------------

local function parse_args(arg)
	local config = {}
	local i = 1
	while true do
		local flag = arg[i]
		if not flag then
			break
		elseif flag == '-s' then
			i = i + 1
			config.shadow = arg[i]
			if not config.shadow then
				return nil, ('missing argument after flag -s, expected path to shadow repository in "git-like" format')
			end
		else
			return nil, ('unknown flag: %s'):format(flag)
		end
		i = i + 1
	end
	return config
end

local function write_file(relpath, contents, config)
	nnn.osmkdirp(nnn.ospath(config.shadow .. '/' .. relpath))
	-- TODO: support binary files
	local fh = assert(io.open(nnn.ospath(config.shadow .. '/' .. relpath), 'w'))
	assert(fh:write(contents))
	-- TODO: can below line return error in Lua?
	fh:close()
end

local function git(argline, config)
	-- FIXME: hide output, etc.
	-- FIXME: verify no spaces in 'shadow' path, or otherwise handle it correctly in popen call
	assert(os.execute("git -C " .. nnn.ospath(config.shadow) .. " " .. argline))
end

local function git_lines(argline, config)
	-- FIXME: verify no spaces in 'shadow' path, or otherwise handle it correctly in popen call
	local cmd = "git -C " .. nnn.ospath(config.shadow) .. " " .. argline
	local pipe = assert(io.popen(cmd))
	return function()
		local line, err = pipe:read '*l'
		if err and not line then
			error(err .. " when running: " .. cmd)
		elseif not line then
			pipe:close()
		end
		return line
	end
end

-- git_status returns an iterator over files in the shadow repository that have
-- differences between the working tree and the staging area ("index file"),
-- including untracked files. Entries in the returned list are tables with the
-- following fields:
--  - one-letter 'status': M[odified] / A[dded] / D[eleted] / ? [untracked]
--  - relative 'path' (slash-separated)
--
-- TODO: support whitespace and other nonprintable characters without error
local function git_status(shadow)
	-- FIXME: verify no spaces in 'shadow' path, or otherwise handle it correctly in popen call
	local nextline = git_lines("status --porcelain -uall --ignored --no-renames", {shadow = shadow})
	return function()
		while true do
			local line = nextline()
			if not line then
				return nil  -- finish
			elseif #line < 4 then
				error(('line from git status too short: %q'):format(line))
			elseif line:sub(4,4) == '"' then
				-- TODO: support space & special chars: "If a filename contains
				-- whitespace or other nonprintable characters, that field will
				-- be quoted in the manner of a C string literal: surrounded by
				-- ASCII double quote (34) characters, and with interior
				-- special characters backslash-escaped."
				error(('whitespace and special characters not yet supported in paths: %s'):format(line:sub(4)))
			end
			local f = {
				status = line:sub(2,2),
				path = line:sub(4),
			}
			if not (' MAD?'):find(f.status, 1, true) then
				error(('unexpected status %q from git in line: %q'):format(f.status, line))
			end
			if f.status ~= ' ' then
				return f
			end
		end
	end
end

----------
-- main --
----------

-- TODO: local nnn = require 'nnn'; nnn.exec_with(shadow_dir)
-- TODO: nnn.handler(require 'nnn.winfs')
-- TODO: nnn.handler(require 'nnn.winpath') -- with refreshenv support copied from chocolatey
-- TODO: nnn.handler(require 'nnn.zeroinstall')
-- TODO: nnn.handler(require 'nnn.chocolatey')

local function main(arg)
	-- TODO: option `-f config.lua` (default)
	-- TODO: if shadow directory does not exist, do `mkdir -p` and `git init` for it
	-- TODO: allow "dry run" - to verify what would be done/changed on disk (stop before rendering files to disk)

	-- Parse flags
	local config = or_die(parse_args(arg))
	if not config.shadow then
		die("missing mandatory flag -s")
	end

	-- Verify shadow repo is clean
	local iterfiles = git_status(config.shadow)
	if iterfiles() then
		die(("shadow git repo not clean: %s"):format(config.shadow))
	end

	-- Stage the prerequisites in the git repo
	for k, v in pairs(nnn.prereqs) do
		assert_gitpath(k)
		if v == nnn.absent then
			-- FIXME: don't error if file does not exist in shadow repo
			git("rm -- " .. k, config)
		else
			write_file(k, v, config)
			git("add -- " .. k, config)
		end
	end

	-- For each prerequisite (i.e., "git add/rm"-ed file), fetch
	-- corresponding file from disk into shadow repo, so that we can later
	-- easily compare them for differences. NOTE: we can't account for
	-- 'absent' prereqs here, as we don't have enough info; those will be
	-- checked later.
	for path in git_lines("ls-files --cached", config) do
		nnn.oscopy(nnn.ospath(path), nnn.ospath(config.shadow .. "/" .. path))
	end
	-- Verify that prerequisites match disk contents
	local iterfiles = git_status(config.shadow)
	if iterfiles() then
		die(("real disk contents differ from expected prerequisites; check git diff in shadow repo: %s"):format(config.shadow))
	end

	-- Commit the prerequisites
	-- FIXME: maybe remove --allow-empty, but then skip if `git status` is empty
	git('commit -m "prerequisites" --allow-empty', config)

	-- Any files in git but not in wanted should be removed
	for path in git_lines("ls-tree --name-only -r HEAD", config) do
		if not nnn.wanted[path] then
			nnn.wanted[path] = nnn.absent
		end
	end
	-- Render wanted files
	for k, v in pairs(nnn.wanted) do
		assert_gitpath(k)
		if v == nnn.absent then
			assert(os.remove(nnn.ospath(config.shadow .. '/' .. k)))
		else
			write_file(k, v, config)
		end
	end
	-- For new files, verify they are absent on disk
	for f in git_status(config.shadow) do
		if f.status == '?' and exists(nnn.ospath(f.path)) then
			die("file expected absent, but found on disk: " .. f.path)
		end
	end

	-- Render files to their places on disk!
	for f in git_status(config.shadow) do
		if f.status == '?' then
			nnn.osmkdirp(nnn.ospath(f.path))
		end
		if f.status == '?' or f.status == 'M' then
			nnn.oscopy(nnn.ospath(config.shadow .. '/' .. f.path), nnn.ospath(f.path))
			git("add -- " .. f.path, config)
		elseif f.status == 'D' then
			assert(os.remove(nnn.ospath(f.path)))
			git("rm -- " .. f.path, config)
		else
			die(('unexpected status %q of file in shadow repo: %s'):format(f.status, f.path))
		end
	end

	-- Finalize the deployment
	git('commit -m "deployment" --allow-empty', config)
end

-- FIXME: require -s flag, or set it to some default value
main({
	"-s", "c/prog/shadow"
})

