
local mana = assert(io.popen('mana', 'w'))
mana:write [[
com.akavel.mana.v1
shadow c:\prog\shadow
handle c lua53 handler/winfs.lua 39b9d89e-9522-45a3-a12b-450f027d0bf0
handle path lua53 handler/winpath.lua
handle home lua53 handler/winhome.lua
]]
-- TODO: `handle 0install lua53 handler/zeroinstall.lua`
-- TODO: `handle choco lua53 handler/chocolatey.lua`
-- TODO: add refreshenv support copied from chocolatey

function want(path)
  return function(contents)
    mana:write("want " .. path .. "\n")
    for line in contents:gmatch "[^\n]*" do
      mana:write(" " .. line .. "\n")
    end
  end
end

want 'home/AppData/Local/nvim/init.vim' (require 'vimrc')

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
  lua = "lua53";
}
-- Render oneliners to full files
for name, text in pairs(oneliners) do
  -- TODO: make this work correctly on Linux/Mac
  want("c/bin/" .. name .. ".bat")("@" .. text .. " %*")
end

want "home/.gitconfig" [[
[user]
	email = czapkofan@gmail.com
	name = Mateusz Czapliński
[alias]
	kdiff3 = difftool --tool=kdiff3 --dir-diff
[merge]
	conflictstyle = diff3
]]

want "path/c/bin" ""
want "path/C/Users/Mateusz/.nimble/bin" ""

mana:write [[
affect
]]

