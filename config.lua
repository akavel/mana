local nnn = require 'nnn'
-- TODO: make handlers pluggable external processes, with a configurable
-- command to run each one (managed through popen and json, one line = one
-- command)
nnn.handle('c', require'nnn.winfs'.fordisk'c')
-- TODO: add refreshenv support copied from chocolatey
nnn.handle('path', require 'nnn.winpath')
-- TODO: nnn.handle('0install', require 'nnn.zeroinstall')
-- TODO: nnn.handle('choco', require 'nnn.chocolatey')
nnn.handle('home', require 'nnn.winhome')

require 'vimrc'

-- One-line scripts (~aliases)
local oneliners = {
	gd = "git diff";
	gds = "git diff --staged";
	gf = "git fetch --all";
	gl = "glg --all";
	glg = "git log --graph \"--pretty=format:%%Cred%%h%%Creset -%%C(yellow)%%d%%Creset %%s %%Cgreen(%%cr %%cd) %%C(cyan)%%an%%Creset\" \"--date=format:'%%d.%%m\" --abbrev-commit";
	gs = "git status";
	-- Show git branches in ~ "most recently used" order
	["git-bs"] = "git branch --sort=-committerdate";
}
-- Render oneliners to full files
for name, text in pairs(oneliners) do
	-- TODO: make this work correctly on Linux/Mac
	nnn.wanted["c/bin/" .. name .. ".bat"] = "@" .. text .. " %*"
end

nnn.wanted["path/c/bin"] = ""
nnn.wanted["path/C/Users/Mateusz/.nimble/bin"] = ""

-- Execute --
local winfs = require 'nnn.winfs'
nnn.osmkdirp = winfs.mkdirp
nnn.exec_with(function(path)
	return winfs.ospath(table.concat({'c/prog/shadow', path}, '/'))
end)

