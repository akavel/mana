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

for name, text in pairs(oneliners) do
	if os == "windows" then
		files[bin .. "\\" .. name .. ".bat"] = "@" .. text .. " %*"
	end
end
