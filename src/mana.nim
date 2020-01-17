{.experimental: "codeReordering".}
import "os"
import "osproc"
import "streams"
import "strutils"
import "tables"
import "uri"

when isMainModule:
  main()

# Input protocol: (eggex specification - see: https://www.oilshell.org/release/0.7.pre5/doc/eggex.html)
#
#  HANDSHAKE = / 'com.akavel.mana.v1' CR? LF /
#  SHADOW = / 'shadow ' < URLENCODED = path > CR? LF /
#  HANDLER = / 'handle ' < [a-z 0-9 _]+ = prefix > (' ' < URLENCODED = command_part >)+ CR? LF /
#   LINES = / (' ' < .* = line > CR? LF)* /
#  FILE = / 'want ' < URLENCODED = path > CR? LF LINES /
#  AFFECT = / 'affect' CR? LF /
#
#  INPUT = / HANDSHAKE SHADOW HANDLER+ FILE* AFFECT /

proc main() =
  # TODO: if shadow directory does not exist, do `mkdir -p` and `git init` for it
  # TODO: allow "dry run" - to verify what would be done/changed on disk (stop before rendering files to disk)

  # helpers
  var unread = ""
  proc readLine(): string =
    if unread != "":
      result = unread
      unread = ""
    else:
      result = stdin.readLine
  proc check(cond: bool, errmsg: string) =
    if not cond: die(errmsg)
  proc urldecode(s: string): string =
    decodeUrl(s, decodePlus=false)

  # Read handshake
  var rawline = readLine()
  check(rawline == "com.akavel.mana.v1", "bad first line, expected 'com.akavel.mana.v1', got: '$1'" % rawline)

  # Read shadow repo path
  var line = readLine().split ' '
  check(line.len == 2, "bad shadow line, expected 'shadow ' and urlencoded path, got: '$1'" % line.join " ")
  let shadow = line[1].urldecode.GitRepo

  # Verify shadow repo is clean
  check(shadow.gitStatus().len == 0, "shadow git repo not clean: $1" % shadow.string)

  # Read handler definitions, initialize each handler
  var handlers: Table[string, Process]
  while true:
    line = readLine().split ' '
    check(line.len > 0, "unexpected empty line")
    if line[0] != "handle":
      unread = line.join " "
      break
    check(line.len >= 3, "line missing prefix or command: '$1'" % line.join " ")
    check(not line[1].contains(AllChars - {'a'..'z', '0'..'9', '_'}), "only [a-z0-9_] allowed in prefix, got: '$1'" % line[1])
    var
      prefix = line[1]
      command = line[2].urldecode
      args: seq[string]
    for i in 3 ..< len(line):
      args.add line[i].urldecode
    # TODO: use poDaemon option on Windows
    handlers[prefix] = startProcess(command=command, args=args, options={poUsePath})
    handlers[prefix].inputStream.writeLine "com.akavel.mana.v1.rq"
    let rs = handlers[prefix].outputStream.readLine
    check(rs == "com.akavel.mana.v1.rs", "expected handshake 'com.akavel.mana.v1.rs' from handler '$1', got '$2'" % [command, rs])

  # For each prerequisite (i.e., "git add/rm"-ed file), gather corresponding
  # file from disk into shadow repo, so that we can later easily compare them
  # for differences. NOTE: we can't account for 'absent' prereqs here, as we
  # don't have enough info; those will be checked later.
  for path in shadow.gitLines "ls-files --cached":
    var ph = path.handler
    var shadowpath = shadow.ospath path
    if ph.detect:
      ph.gather_to shadowpath
    else:
      removeFile shadowpath
  # Verify that prerequisites match disk contents
  check(shadow.gitStatus.len == 0, "real disk contents differ from expected prerequisites; check git diff in shadow repo: " & shadow.string)

  # List all files present in shadow repo
  var inshadow = toHashSet(shadow.gitLines "ls-tree --name-only -r HEAD")

  # Read wanted files, and write them to git repo
  while true:
    line = readLine().split ' '
    check(line.len > 0, "expected 'want' line or 'exec' line, got empty line")
    case line[0]
    of "want":
      check(line.len == 3, "expected urlencoded path and 'with'/'absent' after 'want' in: '$1'" % line.join " ")
      let path = line[1].urldecode
      check_gitpath(path)
      # FIXME: [LATER] allow emitting CR-terminated lines (by allowing binary files somehow)
      var fh = open(shadow.ospath path, mode = fmWrite)
      while true:
        let rawline = readLine()
        check(rawline.len > 0, "unexpected empty line in input")
        if rawline[0] != ' ':
          unread = rawline
          break
        fh.writeLine rawline[1..^1]
      fh.close()
      inshadow.excl path
  # Remove from shadow repo any files that are not wanted
  for path in inshadow:
    removeFile shadow.ospath path
  # For new files, verify they are absent on disk
  for f in shadow.gitStatus:
    check(f.status != '?' or not f.path.handler.detect, "file expected absent, but found on disk: $1" % f.path)

  # Read final 'affect' line
  let rawline = readLine()
  check(rawline == "affect", "expected final 'affect' line, got: '$1'" % rawline)

  # Render files to their places on disk!
  for f in shadow.gitStatus:
    case f.status
    of 'M', '?':
      f.path.handler.affect shadow.ospath(f.path)
      shadow.git "add", "--", f.path
    of 'D':
      f.path.handler.affect shadow.ospath(f.path)
      shadow.git "rm", "--", f.path
    else:
      die "unexpected status $1 of file in shadow repo: $2" % [$f.status, f.path]

  # Finalize the deployment
  shadow.git "commit", "-m", "deployment", "--allow-empty"

proc die(msg: varargs[string, `$`]) =
  stderr.writeLine "error: " & msg.join ""
  quit 1

type
  GitRepo = distinct string  # repo path
  GitFile = distinct string  # relative path of a file in a GitRepo
  GitStatus = tuple[status: char, path: GitFile]

# FIXME: add also a command gitZStrings for NULL-separated strings
# TODO: [LATER]: write an iterator variant of this proc
proc rawGitLines(repo: GitRepo, args: openarray[string]): seq[TaintedString] =
  var p = startProcess("git", workingDir=repo.string, args=args, options={poUsePath})
  var outp = outputStream(p)
  close inputStream(p)
  # TODO: what about errorStream(p) ?
  while not outp.atEnd:
    # FIXME: implement better readLine
    result.add outp.readLine()
  while p.peekExitCode() == -1:
    continue
  close(p)
  if p.peekExitCode() != 0:
    # TODO: [LATER]: better exception type
    raise newException(ValueError, "command 'git " & args.join " " & "' returned non-zero exit code: " & $p.peekExitCode())

proc gitStatus(repo: GitRepo): seq[GitStatus] =
  # FIXME: don't use die in this func, raise exceptions instead
  for line in repo.rawGitLines "status --porcelain -uall --ignored --no-renames":
    if line.len < 4: die "line from git status too short: " & line
    elif: line[3] == '"': die "whitespace and special characters not yet supported in paths: " & line[3..^1]
    let info = (status: line[2], path: line[4..^1])
    if info.status notin " MAD?": die "unexpected status from git in line: " & line  # {' ', 'M', 'A', 'D', '?'}
    if info.status != ' ':
      result.add info

