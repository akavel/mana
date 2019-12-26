-- TODO: move this block into `require "nnn"`, maybe
local nnn = {
	prereqs = {},
	files = {},
	absent = {},  -- special tag value for marking absent files
}

local bin = "c/bin"

local oneliners = {
	gd = "git diff";
	gds = "git diff --staged";
	gs = "git status";
	gf = "git fetch --all";
	glg = "git log --graph \"--pretty=format:%Cred%h%Creset -%C(yellow)%d%Creset %s %Cgreen(%cr %cd) %C(cyan)%an%Creset\" \"--date=format:'%d.%m\" --abbrev-commit";
	gl = "glg --all";
}

-- Render oneliners to full files
for name, text in pairs(oneliners) do
	if os == "windows" then
		nnn.files[bin .. "/" .. name .. ".bat"] = "@" .. text .. " %*"
	end
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

local function assert_gitpath(s)
	if #s == 0 then
		error("empty path")
	end
	local function assert_no(pattern, msg)
		local from, to = s:find(pattern)
		if from then
			error(("%s: found %q at offset %d in path: %q"):(msg, s:sub(from,to), from, s))
		end
	end
	-- FIXME: also sanitize other Windows-specific stuff, like 'nul' or 'con' in filenames
	-- FIXME: properly handle case-insensitive filesystems (Windows, Mac?)
	assert_no("[^%w_-/.]", "TODO: unsupported character")
	-- FIXME: actually *require* absolute paths, then de-absolutize them
	assert_no("^/", "absolute path")
	assert_no("/../", "relative path")
	assert_no("^../", "relative path")
	assert_no("/..$", "relative path")
	assert_no("^..$", "relative path")
	assert_no("/./", "denormalized path")
	assert_no("^./", "denormalized path")
	assert_no("/.$", "denormalized path")
	assert_no("^.$", "denormalized path")
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
				return nil, ('missing argument after flag -s, expected path to shadow repository')
			end
		else
			return nil, ('unknown flag: %s'):format(flag)
		end
		i = i + 1
	end
	return config
end

local function write_file(relpath, contents, config)
	-- TODO: mkdir -p $SHADOW/$(dirname relpath)
	-- TODO: support binary files
	local fh = assert(io.open(config.shadow .. '/' .. relpath, 'w'))
	assert(fh:write(contents))
	-- TODO: can below line return error in Lua?
	fh:close()
end

local function git(argline, config)
	-- FIXME: hide output, etc.
	assert(os.execute("git -C " .. config.shadow .. " " .. argline))
end

-- git_status returns a list of files in the shadow repository that have
-- differences between the working tree and the staging area ("index file"),
-- including untracked files. The function returns nil and error in case of
-- problems. Entries in the returned list are tables with the following fields:
--  - one-letter 'status': M[odified] / A[dded] / D[eleted] / ? [untracked]
--  - relative 'path' (slash-separated)
--
-- TODO: support whitespace and other nonprintable characters without error
-- TODO: LATER make it return an iterator
local function git_status(shadow)
	-- FIXME: verify no spaces in 'shadow' path, or otherwise handle it correctly in popen call
	local pipe, err = io.popen("git -C " .. shadow .. " status --porcelain -uall --ignored --no-renames")
	if not pipe then
		return nil, err
	end
	local files = {}
	while true do
		local line, err = pipe:read '*l'
		if not line then
			pipe:close()
			if err then
				return nil, err
			end
			break
		end
		if #line < 4 then
			pipe:close()
			return nil, ('line from git status too short: %q'):format(line)
		end
		-- TODO: support space & special chars: "If a filename contains
		-- whitespace or other nonprintable characters, that field will
		-- be quoted in the manner of a C string literal: surrounded by
		-- ASCII double quote (34) characters, and with interior
		-- special characters backslash-escaped."
		if line:sub(4,4) == '"' then
			pipe:close()
			return nil, ('whitespace and special characters not yet supported in paths: %s'):format(line:sub(4))
		end
		local f = {
			status = line:sub(2,2),
			path = line:sub(4),
		}
		if not (' MAD?'):find(f.status, 1, true) then
			pipe:close()
			return nil, ('unexpected status %q from git in line: %q'):format(f.status, line)
		end
		if f.status ~= ' ' then
			files[#files+1] = f
		end
	end
	return files
end

local function want()
	for path, contents in pairs(files) do

	end
end

----------
-- main --
----------

local function main(arg)
	-- TODO: option `-f config.lua` (default)
	-- TOOD: if shadow directory does not exist, do `mkdir -p` and `git init` for it

	-- Parse flags
	local config = or_die(parse_args(arg))
	if not config.shadow then
		die("missing mandatory flag -s")
	end

	-- Verify shadow repo is clean
	local files = or_die(git_status(config.shadow))
	if #files > 0 then
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

	-- For each prerequisite, fetch corresponding file from disk into
	-- shadow repo, so that we can later easily compare them for
	-- differences. NOTE: we can't account for 'absent' prereqs here, as we
	-- don't have enough info; those will be checked later.

	-- TODO: prereqs()   # exec `git add` & `git rm` commands in $SHADOW dir, then `git commit`, then create git ref "nnn/prereqs"
	-- TODO: git manifest refs/nnn/prereqs | foreach line; do copy "$(winpath "$line")" "$SHADOW/$line"; done  # TODO: s/copy/rsync
	-- TODO: git status -C $SHADOW --untracked --no-gitignore || die "Shadow tree git repo not clean on prereqs check"
	-- TODO: want()      # exec `git add` & `git rm` commands in $SHADOW dir, BUT NO `git add/rm/commit`
	-- TODO: git status -C $SHADOW --untracked-only --no-gitignore | foreach line; do [ ! -f "$(winpath "$line")" ] || die "Real file present, expected absent"; done
	-- TODO: git diff --raw --include-untracked | foreach line; do \
	--      case line.action in
	--	added|modified) copy "$SHADOW/$line" "$(winpath "$line")"; git add "$SHADOW/$line";;  # TODO: s/copy/rsync
	--      removed) del "$(winpath "$line")"; git rm "$SHADOW/$line";;
	--      esac
end

-- FIXME: require -s flag, or set it to some default value
main({
	"-s", "c:\\prog\\shadow"
})

