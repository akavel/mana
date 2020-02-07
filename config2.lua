
-- local proc = assert(popen('mana', 'w'))
local proc = assert(io.open('mana.log', 'w'))
proc:write [[
com.akavel.mana.v1
shadow c:\prog\shadow
handle c lua53 mana/winfs.lua c
handle path lua53 mana/winpath.lua
handle home lua53 mana/winhome.lua
]]
-- TODO: `handle 0install lua53 mana/zeroinstall.lua`
-- TODO: `handle choco lua53 mana/chocolatey.lua`
-- TODO: add refreshenv support copied from chocolatey

mana = { wanted = {} }
package.loaded.mana = mana
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
  mana.wanted["c/bin/" .. name .. ".bat"] = "@" .. text .. " %*"
end

mana.wanted["path/c/bin"] = ""
mana.wanted["path/C/Users/Mateusz/.nimble/bin"] = ""

-- Execute --
for n, v in pairs(mana.wanted) do
  proc:write("want " .. n .. "\n")
  for l in v:gmatch "[^\n]*" do
    proc:write(" " .. l .. "\n")
  end
end
proc:write [[
affect
]]

