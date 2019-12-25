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

-- TODO: option `-f config.lua` (default)
-- TODO: option `-s shadow` - path to the shadow repo

local function want()
	for path, contents in pairs(files) do

	end
end

local function main()
	-- TODO: git status -C $SHADOW --untracked --no-gitignore || die "Shadow tree git repo not clean on init"
	-- TODO: prereqs()   # exec `git add` & `git rm` commands in $SHADOW dir, then `git commit`, then create git ref "nnn/prereqs"
	-- TODO: git manifest refs/nnn/prereqs | foreach line; do copy "$(winpath "$line")" "$SHADOW/$line"; done  # TODO: s/copy/rsync
	-- TODO: git status -C $SHADOW --untracked --no-gitignore || die "Shadow tree git repo not clean on prereqs check"
	-- TODO: want()      # exec `git add` & `git rm` commands in $SHADOW dir, BUT NO `git add/rm/commit`
	-- TODO: git status -C $SHADOW --untracked --no-gitignore | foreach line; do [ ! -f "$(winpath "$line")" ] || die "Real file present, expected absent"; done
	-- TODO: git diff --raw --include-untracked | foreach line; do \
	--      case line.action in
	--	added|modified) copy "$SHADOW/$line" "$(winpath "$line")"; git add "$SHADOW/$line";;  # TODO: s/copy/rsync
	--      removed) del "$(winpath "$line")"; git rm "$SHADOW/$line";;
	--      esac
end

