-- TODO: move this block into `require "nnn"`, maybe
local files = {}
local bin = "c:\\bin"

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
		files[bin .. "\\" .. name .. ".bat"] = "@" .. text .. " %*"
	end
end

------------------------------------------------
-- TODO: move the stuff below to proper "nnn" --
------------------------------------------------

-- check if v is non-nil, or print error and exit the app
local function check(v, err, ...)
	if not v then
		io.stderr:write('error: ', err, '\n')
		os.exit(1)
	end
	return v, err, ...
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
		if not (' MAD?'):find(f.status) then
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

local function main(arg)
	-- TODO: option `-f config.lua` (default)
	-- TOOD: if shadow directory does not exist, do `mkdir -p` and `git init` for it

	local config = check(parse_args(arg))
	if not config.shadow then
		check(nil, "missing mandatory flag -s")
	end

	local files = check(git_status(config.shadow))
	for _, f in ipairs(files) do
		print(f.status, f.path)
	end
	-- TODO: git status -C $SHADOW --untracked --no-gitignore || die "Shadow tree git repo not clean on init"
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

