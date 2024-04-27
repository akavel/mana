# USE:
#     nickel export -f raw --field script win-mana.ncl
#
# PLAN:
#  - [x] port mana's config.lua (of Windows laptop No.1)
#  - [x] refactor to have trees, and only then convert them to 'wants'
#  - [ ] see akavel/drafts

let module = {
  main = {
    "Mateusz@desktop-ssng4gi" =
      common & {
        handlers.c =
          "lua53 handler/winfs.lua 39b9d89e-9522-45a3-a12b-450f027d0bf0",
      },
    "Mateusz@desktop-4k5pnqi" =
      common & {
        handlers.c =
          "lua53 handler/winfs.lua d93d71ef-8b72-4351-a16a-cd2babb6630c",
      },
  },

  common = {
    shadow_dir = m%"c:\prog\shadow"%,
    handlers = handlers',
    tree = tree',
  },

  handlers' = {
# TODO[LATER]: split by whitespace
    c | default = "lua53 handler/winfs.lua FIXME-UUID",
    path = "lua53 handler/winpath.lua",
    home = "lua53 handler/winhome.lua",
    zeroinstall = "zeroinstall",
# TODO[LATER]: `handle 0install lua53 handler/zeroinstall.lua`
# TODO[LATER]: `handle choco lua53 handler/chocolatey.lua`
# TODO[LATER]: add refreshenv support copied from chocolatey
  },

  tree' = {
    c = {
      bin = oneliners |> oneliners_to_bat,
      fotki.tapetki-windows."00-fetch.bat" = win-spotlight-save,
    },
    path = {
      c.bin = "",
      c.Users.Mateusz.".nimble".bin = "",
    },
    # TODO: should merge into `tree` based on value of $HOME
    home = {
      ".gitconfig" = gitconfig,
      AppData.Local.nvim."init.vim" = import "vimrc.txt",
    },
    zeroinstall = by-path {
      "https/akavel.github.io/0catalog/feeds/adb.xml" = zeroinst {
        timestamp = "1702245477",
        aliases = [
          { name = "adb", command = "run" },
        ],
      },
      "https/akavel.github.io/0catalog/feeds/symphytum.xml" = zeroinst {
        timestamp = "1702245476",
        menu-entries = [
          { name = "Symphytum", command = "run", category = "Office" },
        ],
      },
      "http/0install.de/feeds/SevenZip_CLI.xml" = zeroinst {
        timestamp = "1702245476",
        aliases = [
          { name = "7za", command = "run" },
        ],
      },
    },
  },

# TODO: make a contract for the contents
# One-line scripts (~aliases)
  oneliners = {
    gd = "git diff",
    gds = "git diff --staged",
    gf = "git fetch --all",
    gl = "glg --all",
    glg = "git log --graph \"--pretty=format:%%Cred%%h%%Creset -%%C(yellow)%%d%%Creset %%s %%Cgreen(%%cr %%cd) %%C(cyan)%%an%%Creset\" \"--date=format:'%%d.%%m\" --abbrev-commit",
    gs = "git status",
    # Show git branches in ~ "most recently used" order
    git-bs = "git branch --sort=-committerdate",
    lua = "lua53",
  },

  gitconfig = m%"
    [user]
    	email = czapkofan@gmail.com
    	name = Mateusz Czapliński
    [alias]
    	kdiff3 = difftool --tool=kdiff3 --dir-diff
    [merge]
    	conflictstyle = diff3

    [init]
    	defaultBranch = main
    [core]
    	autocrlf = input

    "%,

  oneliners_to_bat = fun record => record
    |> std.record.to_array
    |> std.array.map (fun x => {
        field = "%{x.field}.bat",
        value = "@%{x.value} %*\n",
      })
    |> std.record.from_array,

  win-spotlight-save = m%%%%"
    @echo off
    setlocal
    :: https://www.groovypost.com/howto/save-windows-10-spotlight-lock-screen-pictures/
    :: https://stackoverflow.com/questions/7881035/checking-file-size-in-a-batch-script

    set here=%CD%
    cd /d %userprofile%\AppData\Local\Packages\Microsoft.Windows.ContentDeliveryManager_*\LocalState\Assets

    set minsize=200000

    for %%F in (*.*) do (
        if %%~zF GTR %minsize% (
            echo %%F
            if not exist %here%\%%F.jpg (
                copy /b %%F %here%\%%F.jpg
            ) else (
                echo ...exists
            )
        )
    )
    cd /d %here%
    pause

    endlocal
    "%%%%,

  # TODO: typing
  # TODO: support more features
  zeroinst = fun app =>
    let menu-entries = get-or app "menu-entries" []
      |> std.array.map (fun x =>
        "<menu-entry name=\"%{x.name}\" command=\"%{x.command}\" category=\"%{x.category}/%{x.name}\" />") in
    let aliases = get-or app "aliases" []
      |> std.array.map (fun x =>
        "<alias name=\"%{x.name}\" command=\"%{x.command}\" />") in
    let body = ["<capability-registration />"] @ menu-entries @ aliases
        |> std.string.join "\n"
    in m%"
      <app timestamp="%{ app.timestamp }">
        <access-points xmlns="http://0install.de/schema/desktop-integration/app-list" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
          %{body}
        </access-points>
      </app>
    "%,

  by-path = fun record-with-paths => record-with-paths
    |> std.record.to_array  # [{field=..., value=...}...]
    |> std.array.map (fun {field, value} => field |> std.string.split "/" |> put-deep value)
    |> std.record.merge_all,

  put-deep = fun value path =>
    path |> std.array.fold_right (fun it acc => std.record.insert it acc {}) value,

  get-or = fun record field-name fallback =>
    if std.record.has_field field-name record
      then record."%{field-name}"
      else fallback,

} in
module.main

