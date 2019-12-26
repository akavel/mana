local nnn = require 'nnn'
-- TODO: nnn.handle('c', ...)
nnn.handle(require 'nnn.winfs')
-- TODO: nnn.handle('path', require 'nnn.winpath') -- with refreshenv support copied from chocolatey
-- TODO: nnn.handle('0install', require 'nnn.zeroinstall')
-- TODO: nnn.handle('choco', require 'nnn.chocolatey')

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
	-- TODO: make this work correctly on Linux/Mac
	nnn.wanted["c/bin/" .. name .. ".bat"] = "@" .. text .. " %*"
end

local winfs = require 'nnn.winfs'
nnn.osmkdirp = winfs.mkdirp
nnn.exec_with(function(path)
	return winfs.ospath(table.concat({'c/prog/shadow', path}, '/'))
end)

