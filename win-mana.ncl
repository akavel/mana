# USE:
#     nickel export -f raw win-mana.ncl
#
# GOAL:
# 
# com.akavel.mana.v1
# shadow c:\prog\shadow
# handle c lua53 handler/winfs.lua 39b9d89e-9522-45a3-a12b-450f027d0bf0
# handle path lua53 handler/winpath.lua
# handle home lua53 handler/winhome.lua
# want home/AppData/Local/nvim/init.vim (require 'vimrc')

# TODO: make a contract for the contents
let oneliners = {
  gd = "git diff",
  gds = "git diff --staged",
  gf = "git fetch --all",
  gl = "glg --all",
  glg = "git log --graph \"--pretty=format:%%Cred%%h%%Creset -%%C(yellow)%%d%%Creset %%s %%Cgreen(%%cr %%cd) %%C(cyan)%%an%%Creset\" \"--date=format:'%%d.%%m\" --abbrev-commit",
  gs = "git status",
  # Show git branches in ~ "most recently used" order
  git-bs = "git branch --sort=-committerdate",
  lua = "lua53",
} in

# TODO[LATER]: why doesn't work with `fun path contents => ...` ?
let want = fun p c => { "path" = p, "contents" = c } in

let wants = [
  (want "home/AppData/Local/nvim/init.vim" (import "vimrc.txt"))
] in

# TODO: urlencode path
let wantify = fun want => m%"
want %{want.path}
  %{want.contents |> std.string.replace "\n" "\n "}
"% in

m%"
com.akavel.mana.v1
shadow c:\prog\shadow
handle c lua53 handler/winfs.lua 39b9d89e-9522-45a3-a12b-450f027d0bf0
handle path lua53 handler/winpath.lua
handle home lua53 handler/winhome.lua
%{wants |> std.array.map wantify |> std.string.join "\n" }
"%
