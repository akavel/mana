# USE:
#     nickel export -f raw win-mana.ncl
#
# PLAN:
#  - [ ] port mana's config.lua (of Windows laptop No.1)
#  - [ ] refactor to have trees, and only then convert them to 'wants'
#  - [ ] see akavel/drafts

let shadow_dir = m%"c:\prog\shadow"% in

let handlers = {
  c = "lua53 handler/winfs.lua 39b9d89e-9522-45a3-a12b-450f027d0bf0",
  path = "lua53 handler/winpath.lua",
  home = "lua53 handler/winhome.lua",
} in

# TODO[LATER]: `handle 0install lua53 handler/zeroinstall.lua`
# TODO[LATER]: `handle choco lua53 handler/chocolatey.lua`
# TODO[LATER]: add refreshenv support copied from chocolatey

# TODO: make a contract for the contents
# One-line scripts (~aliases)
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
let want = fun p c => { path = p, contents = c } in

let wants = [
#  want "home/AppData/Local/nvim/init.vim" (import "vimrc.txt"),
  want "path/c/bin" "",
  want "path/C/Users/Mateusz/.nimble/bin" "",
] @ (
  oneliners |> std.record.to_array |> std.array.map (fun x => {
    path = "c/bin/%{x.field}.bat",
    contents = "@%{x.value} %*"
  })
) in

# TODO: urlencode path
let wantify = fun w =>
    let body = if std.record.has_field "contents" w && w.contents != ""
      then "\n %{w.contents |> std.string.replace "\n" "\n "}"
      else ""
    in "want %{w.path}%{body}"
  in

let handlify = fun h =>
  m%"handle %{h.field} %{h.value}"% in

m%"
com.akavel.mana.v1
shadow %{shadow_dir}
%{ handlers |> std.record.to_array |> std.array.map handlify |> std.string.join "\n" }
%{ wants |> std.array.map wantify |> std.string.join "\n" }
"%
