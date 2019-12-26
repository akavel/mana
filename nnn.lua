local nnn = {
	absent = {},  -- special tag value for marking absent files
	handlers = {},  -- will contain a list of handlers to use for processing paths

	-- below values should be filled by user

	prereqs = {},
	wanted = {},
	-- osmkdirp should create all parent directories leading to the
	-- specified OS path. This should be equivalent to Linux command:
	-- `mkdir -p "$(dirname "$path")"`; currently, example handler for
	-- Windows OS is provided
	osmkdirp = function(path)
		local iter = path:gmatch "([^\\]+)\\"
		local parent = iter()  -- "C:" or similar
		for d in iter do
			parent = parent .. '\\' .. d
			os.execute("mkdir " .. parent)
		end
	end,
}

local function write_file(ospath, contents)
	-- TODO: remove dependency on osmkdirp
	nnn.osmkdirp(ospath)
	-- TODO: support binary files
	local fh = assert(io.open(ospath, 'w'))
	assert(fh:write(contents))
	-- TODO: can below line return error in Lua?
	fh:close()
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

-- TODO: to remove need for `git -C` (and simplify handling of spaces in there), require calling nnn from working dir inside shadow repo

local function git(argline, shadowf)
	-- FIXME: hide output, etc.
	-- FIXME: verify no spaces in 'shadow' path, or otherwise handle it correctly in popen call
	assert(os.execute("git -C " .. shadowf() .. " " .. argline))
end

local function git_lines(argline, shadowf)
	-- FIXME: verify no spaces in 'shadow' path, or otherwise handle it correctly in popen call
	local cmd = "git -C " .. shadowf() .. " " .. argline
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
local function git_status(shadowf)
	local nextline = git_lines("status --porcelain -uall --ignored --no-renames", shadowf)
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

local function handler(path)
	for _, h in ipairs(nnn.handlers) do
		if h.owns(path) then
			return h
		end
	end
	error(('no handler found for path: %q'):format(path))
end


-- TODO: local nnn = require 'nnn'; nnn.exec_with(shadow_dir)
-- TODO: nnn.handler(require 'nnn.winfs')
-- TODO: nnn.handler(require 'nnn.winpath') -- with refreshenv support copied from chocolatey
-- TODO: nnn.handler(require 'nnn.zeroinstall')
-- TODO: nnn.handler(require 'nnn.chocolatey')

-- NOTE: shadowf must be a function converting a slash-separated relative path
-- to an absolute path in 'shadow' git repository; it must return path to git
-- repository when passed empty or nil argument
function nnn.exec_with(shadowf)
	-- TODO: if shadow directory does not exist, do `mkdir -p` and `git init` for it
	-- TODO: allow "dry run" - to verify what would be done/changed on disk (stop before rendering files to disk)

	-- Verify shadow repo is clean
	local iterfiles = git_status(shadowf)
	if iterfiles() then
		error(("shadow git repo not clean: %s"):format(shadowf()))
	end

	-- Stage the prerequisites in the git repo
	for k, v in pairs(nnn.prereqs) do
		assert_gitpath(k)
		if v == nnn.absent then
			-- FIXME: don't error if file does not exist in shadow repo
			git("rm -- " .. k, shadowf)
		else
			write_file(shadowf(k), v)
			git("add -- " .. k, shadowf)
		end
	end

	-- For each prerequisite (i.e., "git add/rm"-ed file), fetch
	-- corresponding file from disk into shadow repo, so that we can later
	-- easily compare them for differences. NOTE: we can't account for
	-- 'absent' prereqs here, as we don't have enough info; those will be
	-- checked later.
	for path in git_lines("ls-files --cached", shadowf) do
		handler(path).query(path, shadowf)
	end
	-- Verify that prerequisites match disk contents
	local iterfiles = git_status(shadowf)
	if iterfiles() then
		error(("real disk contents differ from expected prerequisites; check git diff in shadow repo: %s"):format(shadowf()))
	end

	-- Commit the prerequisites
	-- FIXME: maybe remove --allow-empty, but then skip if `git status` is empty
	git('commit -m "prerequisites" --allow-empty', shadowf)

	-- Any files in git but not in wanted should be removed
	for path in git_lines("ls-tree --name-only -r HEAD", shadowf) do
		if not nnn.wanted[path] then
			nnn.wanted[path] = nnn.absent
		end
	end
	-- Render wanted files
	for k, v in pairs(nnn.wanted) do
		assert_gitpath(k)
		if v == nnn.absent then
			assert(os.remove(shadowf(k)))
		else
			write_file(shadowf(k), v)
		end
	end
	-- For new files, verify they are absent on disk
	for f in git_status(shadowf) do
		if f.status == '?' and handler(f.path).exists(f.path) then
			error("file expected absent, but found on disk: " .. f.path)
		end
	end

	-- Render files to their places on disk!
	for f in git_status(shadowf) do
		if f.status == '?' or f.status == 'M' then
			handler(f.path).apply(f.path, shadowf) -- TODO: must `mkdir -p` if needed
			git("add -- " .. f.path, shadowf)
		elseif f.status == 'D' then
			handler(f.path).apply(f.path, shadowf) -- TODO: must `rm` if absent in git
			git("rm -- " .. f.path, shadowf)
		else
			error(('unexpected status %q of file in shadow repo: %s'):format(f.status, f.path))
		end
	end

	-- Finalize the deployment
	git('commit -m "deployment" --allow-empty', shadowf)
end

-- TODO: errorf helper func

return nnn

