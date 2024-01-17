
-- Other .vimrc settings, not plugin-related
-- Some of them scavenged or cargo-culted from various random places
-- %localappdata%/nvim/init.vim, based on https://github.com/equalsraf/neovim-qt/issues/68#issuecomment-163556972
return [[

" Load plugins from $VIMPATH, if available
" (+ workaround for broken handling of packpath by vim8/neovim for ftplugins)
" TODO: handle `helptags` somehow; consider
let g:pathsep = ':'
if has("win32")
    let g:pathsep = ';'
endif
filetype off | syn off
for p in split($VIMPATH,g:pathsep)
    let &rtp = p . ',' . &rtp . ',' . p . '/after'
endfor
"echom &rtp
filetype indent plugin on | syn on

" I don't want clicking the mouse to enter visual mode:
set mouse-=a

" autoindent etc
set cindent
set smartindent
set autoindent
set expandtab           " replace TABs with spaces
set tabstop=4
set shiftwidth=4

" vimdiff vs. non-vimdiff
if &diff
  " options for vimdiff
  " with syntax highlighting, colors are often unreadable in vimdiff
  syntax off
  " ignore whitespace
  set diffopt+=iwhite
else
  " options for regular vim, non-vimdiff
  syntax on
endif

" disable ZZ (like :wq) to avoid it when doing zz with caps-lock on
nnoremap ZZ <Nop>

" folding: create foldpoints, but unfold by default
set foldlevel=99
augroup vimrc
  " from: http://vim.wikia.com/wiki/Folding
  " create folds based on indent...
  au BufReadPre * setlocal foldmethod=indent
  " ...but allow manual folds too
  au BufWinEnter * if &fdm == 'indent' | setlocal foldmethod=manual | endif
augroup END

" double backspace -> wrap all windows
"nmap <BS>     :set wrap!<CR>
nmap <BS><BS> :windo set wrap!<CR>

" Enable code folding in Go (and others)
" http://0value.com/my-Go-centric-Vim-setup
" Note:
" zM - close all
" zR - open all
" zc - close current fold
" zo - open current fold
set foldmethod=syntax
"set foldmethod=indent
set foldnestmax=10
set nofoldenable
set foldlevel=10

" some undo/backup settings
if has('unix')
  set backup      " keep a backup file (restore to previous version)
  set undofile    " keep an undo file (undo changes after closing)
  " don't litter current dir with backups, but still try to put them
  " somewhere; double slash // at the end stores filenames with path
  set backupdir-=.
  set backupdir^=~/tmp//,/tmp//
  set undodir-=.
  set undodir^=~/tmp//,/tmp//
endif

" Don't use Ex mode, use Q for formatting
map Q gq

" CTRL-U in insert mode deletes a lot.  Use CTRL-G u to first break undo,
" so that you can undo CTRL-U after inserting a line break.
inoremap <C-U> <C-G>u<C-U>

set encoding=utf-8
set fileencoding=utf-8

" Extending the % ("go to pair") key
runtime macros/matchit.vim

" when splitting window (with C-W,v or C-W,s), open to right/bottom
set splitright
set splitbelow

" bash-like (or, readline-like) tab completion of paths, case insensitive
set wildmode=longest,list,full
set wildmenu
if exists("&wildignorecase")
  set wildignorecase
endif

" Show tabs and trailing whitespace visually
set list
set listchars=
set listchars+=tab:¸·
set listchars+=trail:×
" helpful options for :set nowrap
set listchars+=precedes:«
set listchars+=extends:»
set sidescroll=5

" In command-line, use similar navigation keys like in bash/readline
" http://stackoverflow.com/a/6923282/98528
" Note: <C-f> switches to "full editing" of commandline, <C-c> back
cnoremap <C-a> <Home>
cnoremap <C-e> <End>
cnoremap <C-p> <Up>
cnoremap <C-n> <Down>
cnoremap <C-b> <Left>
cnoremap <C-f> <Right>
cnoremap <M-b> <S-Left>
cnoremap <M-f> <S-Right>

" Set 'git grep' as default command for :grep
" This is much faster and more featureful when available. And I'm using it
" mostly only when it's indeed available. If I were to use normal grep, I'd
" anyway by default go for :cex system('grep ...')
set grepprg=git\ grep\ -n

" Use single space ' ' instead of double '  ' after end of sentence
" in gq and J commands. See: https://stackoverflow.com/a/4760477/98528
set nojoinspaces

" display more info on status line
set showmode            " show mode in status bar (insert/replace/...)
set titlestring=%{hostname()}::\ \ %t%(\ %M%)%(\ (%{expand(\"%:p:h\")})%)%(\ %a%)\ \-\ VIM
let &titleold=hostname()
set title               " show file in titlebar
if &term == "screen"    " for tmux, http://superuser.com/a/279214/12184
  set t_ts=ESCk
  set t_fs=ESC\
  set titlestring=%t%(\ %M%)%(\ (%{expand(\"%:p:h\")})%)%(\ %a%)
endif

" modeline: allows settings tweaking per edited file, by special magic comment
" starting with 'vim:'
"set modeline

"" Copied from Ubuntu 16.04 /usr/share/vim/vim74/debian.vim
"" We know xterm-debian is a color terminal
"if &term =~ "xterm-debian" || &term =~ "xterm-xfree86"
"  set t_Co=16
"  set t_Sf=ESC[3%dm
"  set t_Sb=ESC[4%dm
"endif
"let &t_Co=256

" Set color scheme. This one matched the vim 8.1's "default" that I was randomly (?)
" assigned on Ubuntu/Nix, and liked enough.
" Note: for testing, use:
"  :so $VIMRUNTIME/syntax/colortest.vim  " test colors palette
"  :so $VIMRUNTIME/syntax/hitest.vim     " test currently chosen color scheme
" TODO(akavel): find out what I'm using at work, and also do more research on
" colorschemes; make sure that comments look readable
if has('unix')  " TODO: change to 'if not nvim-qt' somehow
  colorscheme ron
endif

" This trigger takes advantage of the fact that the quickfix window can be
" easily distinguished by its file-type, qf. The wincmd J command is
" equivalent to the Ctrl+W, Shift+J shortcut telling Vim to move a window to
" the very bottom (see :help :wincmd and :help ^WJ).
autocmd FileType qf wincmd J

" *.md is for Markdown, not Modula (sorry!)
" http://stackoverflow.com/a/14779012/98528 etc.
au BufNewFile,BufFilePre,BufRead *.md set filetype=markdown

" For *.plant files, use filetype "conf"
au BufNewFile,BufFilePre,BufRead *.plant set filetype=conf

augroup akavel_go
  autocmd!
  " Custom key mappings:
  " gD - go to Definition in split window
  " gle - Go caLleEs
  " glr - Go caLleRs
  autocmd FileType go nmap <buffer> <silent> gD <Plug>(go-def-vertical)
  autocmd FileType go nmap <buffer> <silent> gle <Plug>(go-callees)
  autocmd FileType go nmap <buffer> <silent> glr <Plug>(go-callers)
  " vim-go:
  " `:A`  -- goes to $(FILE)_test.go and back
  " `:A!` -- above, even if target doesn't exist
  " `:AV` etc. -- as :A but in vertical split
  autocmd FileType go command! -bang A call go#alternate#Switch(<bang>0, 'edit')
  autocmd FileType go command! -bang AV call go#alternate#Switch(<bang>0, 'vsplit')
  autocmd FileType go command! -bang AS call go#alternate#Switch(<bang>0, 'split')

  " Remove some unexplicable ignore patterns originating from otherwise
  " awesome vim-go plugin:
  " - ignoring comment-like lines - regexp: '# .*'
  autocmd FileType go setlocal errorformat-=%-G#\ %.%#
  " - ignoring panics (why the hell? :/)
  autocmd FileType go setlocal errorformat-=%-G%.%#panic:\ %m
  " - ignoring empty lines
  autocmd FileType go setlocal errorformat-=%-G%.%#
  " TODO(mateuszc): wtf is the pattern below?
  autocmd FileType go setlocal errorformat-=%C%*\\\s%m

  " Add patterns for gometalinter
  autocmd FileType go setlocal errorformat+=%f:%l:%c:%t%*[^:]:\\\ %m,%f:%l::%t%*[^:]:\\\ %m

  " Add patterns for various Go output formats. (Esp. stacktraces in panics
  " and data race reports.)
  " autocmd FileType go setlocal errorformat+=%A%>%m:,%Z\\\ \\\ \\\ \\\ \\\ %f:%l\\\ +%.%#,%+C\\\ \\\ %m,%A\\\ \\\ %m
  " MATCH:      /path/to/some/file.go:32 +0x23c2
  autocmd FileType go setlocal errorformat+=%Z\\\ \\\ %#%f:%l\\\ +%.%#
  " autocmd FileType go setlocal errorformat+=%A%>%m:
  autocmd FileType go setlocal errorformat+=%+A%>panic:\\\ %.%#
  " autocmd FileType go setlocal errorformat+=%Z\\\ \\\ \\\ \\\ \\\ %f:%l\\\ +%.%#
  " autocmd FileType go setlocal errorformat+=%+C\\\ \\\ %m
  " autocmd FileType go setlocal errorformat+=%A\\\ \\\ %m
  " MATCH: goroutine 123 [some status]:
  " autocmd FileType go setlocal errorformat+=%+A%>goroutine\\\ %[0-9]%[0-9]%#\\\ [%.%#]:
  autocmd FileType go setlocal errorformat+=%+Agoroutine\\\ %.%#
  " MATCH: created by gopackage.goFuncName
  autocmd FileType go setlocal errorformat+=%+Acreated\\\ by\\\ %[a-z]%.%#.%.%#
  " MATCH: path/to/go/package.funcName(0xf00, 0xba4)
  " MATCH: path/to/go/package.(*object).funcName(0xf00, 0xba4, ...)
  autocmd FileType go setlocal errorformat+=%+A\\\ %#%[a-z]%.%#.%.%#(%[0-9a-fx\\\,\\\ %.]%#)

  "autocmd FileType go let &colorcolumn="80,".join(range(120,299),",")
  autocmd FileType go let &colorcolumn="80,120"
  autocmd FileType go highlight ColorColumn ctermbg=darkgray guibg=#f8f8f8

  autocmd FileType go setlocal foldmethod=syntax
  autocmd FileType go setlocal foldlevelstart=99

  " TODO(akavel): why this is not already enabled???
  "autocmd FileType go setlocal noexpandtab
  " NOTE(akavel): alternatively, try 'zR' instead of foldlevelstart above?
augroup END

" Use tabs in .proto files
augroup akavel_proto
  autocmd!
  autocmd FileType proto setlocal expandtab
augroup end

" Fix reflow of text (gq) when editing git commit messages
augroup akavel_git
  autocmd!
  autocmd FileType gitcommit setlocal nocindent
augroup end

" Improved alternatives to :bufdo, :windo, :tabdo (go back to current view
" after finished).
" Source: http://vim.wikia.com/wiki/Run_a_command_in_multiple_buffers
"
" Like windo but restore the current window.
" Like windo but restore the current window.
function! WinDo(command)
  let curwin=winnr()
  let altwin=winnr('#')
  execute 'windo ' . a:command
  execute altwin . 'wincmd w'
  execute curwin . 'wincmd w'
endfunction
com! -nargs=+ -complete=command Windo call WinDo(<q-args>)
" Like bufdo but restore the current buffer.
function! BufDo(command)
  let curbuf=bufnr("%")
  let altbuf=bufnr("#")
  execute 'bufdo ' . a:command
  execute 'buffer ' . altbuf
  execute 'buffer ' . curbuf
endfunction
com! -nargs=+ -complete=command Bufdo call BufDo(<q-args>)
" Like tabdo but restore the current tab.
function! TabDo(command)
  let currTab=tabpagenr()
  execute 'tabdo ' . a:command
  execute 'tabn ' . currTab
endfunction
com! -nargs=+ -complete=command Tabdo call TabDo(<q-args>)

" Convenient command to see the difference between the current buffer and the
" file it was loaded from, thus the changes you made.
" Only define it when not defined already.
if !exists(":DiffOrig")
  command DiffOrig vert new | set bt=nofile | r ++edit # | 0d_ | diffthis
      \ | wincmd p | diffthis
endif
]]
