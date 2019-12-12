 - shortcut scripts in c:\bin
   - l ~ dir
   - gs ~ git status
   - gd ~ git diff
   - gl, glg
   - gf
   * on Windows: in c:\bin, with .bat suffix
   * on Linux/Mac: in $HOME/bin, with chmod +x
 - c:\bin in PATH (also, info about refreshenv, OR ping all cmd
   windows/instances to do refreshenv automatically)
 - choco install:
   - git version X.Y.Z
   - neovim version X.Y.Z
   - go version X.Y.Z
     - info if there are upgrades available
 - unpack and add selected binary to PATH:
   * with hash checked
   - https://github.com/akiyosi/goneovim/releases/download/v0.4.1/goneovim-0.4.1-win64.zip
     - LATER: or other neovim GUI
 - add a neovim GUI tile in Windows Start Menu
 - for every action:
   - if it is already done (output perfectly matches expected
     output), don't redo it (though this logic can be deferred
     onto specific executor)
   - if not already done, verify that current state perfectly
     matches previous generation's state in this area merged
     with "before/previously" fields of current generation
     - if not, report current state into git (as a separate
       commit/ref)


