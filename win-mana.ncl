# USE:
#     nickel export -f raw --field script win-mana.ncl
#
# PLAN:
#  - [x] port mana's config.lua (of Windows laptop No.1)
#  - [x] refactor to have trees, and only then convert them to 'wants'
#  - [ ] see akavel/drafts

let shadow_dir' = m%"c:\prog\shadow"% in

let handlers' = {
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

let gitconfig = m%"
[user]
	email = czapkofan@gmail.com
	name = Mateusz Czapliński
[alias]
	kdiff3 = difftool --tool=kdiff3 --dir-diff
[merge]
	conflictstyle = diff3
"% in

let oneliners_to_bat = fun record => record
  |> std.record.to_array
  |> std.array.map (fun x => {
      field = "%{x.field}.bat",
      value = "@%{x.value} %*",
    })
  |> std.record.from_array
  in

let tree' = {
  c = {
    bin = oneliners |> oneliners_to_bat,
  },
  path = {
    c.bin = "",
    C.Users.Mateusz.".nimble".bin = "",
  },
  # TODO: should merge into `tree` based on value of $HOME
  home = {
    ".gitconfig" = gitconfig,
    AppData.Local.nvim."init.vim" = import "vimrc.txt",
  },
} in

{
  raw = {
    tree = tree',
    shadow_dir = shadow_dir',
    handlers = handlers',
  },
  script =
    raw.tree |> (import "lib/mana.ncl").windows raw.shadow_dir raw.handlers,
}

