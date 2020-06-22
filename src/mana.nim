{.experimental: "codeReordering".}
import options
import os
import osproc
import sequtils
import sets
from sugar import dup
import streams
import strutils
import tables
import terminal
import unicode
import uri

import npeg
import result

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

grammar "input":
  STREAM <-
    Handshake *
    Shadow *
    +Handler *
    *File *
    Affect

  Handshake <- "com.akavel.mana.v1" * eol
  Shadow    <- "shadow " * >urlencoded * eol
  Handler   <- "handle " * >simple_word * +(" " * >urlencoded) * eol
  File      <- file_name * (*file_line)
  file_name <- "want " * >urlencoded * eol
  file_line <- " " * >(*char) * eol
  Affect    <- "affect" * eol


  eol <- ?"\r" * "\n"  # FIXME: is "\n" correct when compiled on Windows?
  urlencoded <- +(
    ("%" * hex * hex) |
    Alpha | Digit | "-" | "." | "_" | "~")  # RFC 3986 2.3.
  hex <- {'0'..'9', 'a'..'f', 'A'..'F'}
  simple_word <- +{'a'..'z', '0'..'9', '_'}
  char <- 1 - {'\r', '\n'}


when isMainModule:
  main()


# FIXME: use custom TaintedString (original one breaks stdlib too much)

func `==`(t: TaintedString, s: string): bool =
  t.string == s
func split[T](t: TaintedString, sep: T): seq[TaintedString] =
  t.string.split(sep).seq[:TaintedString]
func join(t: seq[TaintedString], sep: string): TaintedString =
  t.seq[:string].join(sep).TaintedString

proc main() =
  # TODO: if shadow directory does not exist, do `mkdir -p` and `git init` for it
  # TODO: allow "dry run" - to verify what would be done/changed on disk (stop before rendering files to disk)

  system.addQuitProc(resetAttributes)

  # helpers
  var unread = "".TaintedString
  proc readLine(): TaintedString =
    if unread != "":
      result = unread
      unread = "".TaintedString
    else:
      result = stdin.readLine & "\n"

  # Read handshake
  let handshake = readLine() =~ input.Handshake
  CHECK(handshake.isOk, "bad first line, expected 'com.akavel.mana.v1', got: '$1'", handshake.error)

  # Read shadow repo path
  let rawShadow = readLine() =~ input.Shadow
  CHECK(rawShadow.isOk, "bad shadow line, expected 'shadow ' and urlencoded path, got: '$1'", rawShadow.error)
  let shadow = rawShadow.get[0].urldecode.GitRepo

  # Verify shadow repo is clean
  CHECK(shadow.gitStatus().len == 0, "shadow git repo not clean: $1", shadow)

  # Read handler definitions, initialize each handler
  var handlers: Table[string, Handler]
  while true:
    let rawHandler = readLine() =~ input.Handler
    if not rawHandler.isOk:
      unread = rawHandler.error & "\n"
      break
    let
      prefix = rawHandler.get[0]
      command = rawHandler.get[1].urldecode
      args = rawHandler.get[2..^1].map(urldecode)
    LOG "handler " & prefix & " " & command & " " & args.join " "
    handlers[prefix] = startHandler(command, args)
  proc toHandler(path: GitFile): tuple[h: Handler, p: GitSubfile] =
    # echo "-", path.string
    let sep = path.string.find '/'
    CHECK(sep > 0, "invalid path (missing slash or empty prefix): $1", path)
    let subpath = path.string[sep+1..^1].GitSubfile
    return (handlers[path.string[0..<sep]], subpath)

  # For each prerequisite (i.e., "git add/rm"-ed file), gather corresponding
  # file from disk into shadow repo, so that we can later easily compare them
  # for differences. NOTE: we can't account for 'absent' prereqs here, as we
  # don't have enough info; those will be checked later.
  for path in shadow.gitFiles("ls-files", "--cached"):
    var ph = path.toHandler
    var shadowpath = shadow.ospath path
    if ph.detect:
      ph.gather_to shadowpath
    else:
      removeFile shadowpath
  # Verify that prerequisites match disk contents
  CHECK(shadow.gitStatus.len == 0, "real disk contents differ from expected prerequisites; check git diff in shadow repo: " & shadow.string)

  # List all files present in shadow repo
  # FIXME: [LATER]: how to make inshadow work as Table[GitFile]?
  var inshadow = shadow.gitFiles("ls-tree", "--name-only", "-r", "HEAD").mapIt((it.string.toLower, it.string)).toTable

  # Read wanted files, and write them to git repo
  while true:
    let rawFile = readLine() =~ input.file_name
    if not rawFile.isOk:
      unread = rawFile.error & "\n"
      break
    let
      path = rawFile.get[0].urldecode.checkGitFile
      ospath = shadow.ospath path
    createDir(ospath.parentDir)
    # FIXME: [LATER] allow emitting CR-terminated lines (by allowing binary files somehow)
    var fh = open(ospath, mode = fmWrite)
    while true:
      let rawdata = readLine() =~ input.file_line
      if not rawdata.isOk:
        unread = rawdata.error & "\n"
        break
      fh.writeLine rawdata.get[0].string
    fh.close()
    inshadow.del path.string.toLower

  # Verify final 'affect' line
  let rawAffect = readLine() =~ input.Affect
  CHECK(rawAffect.isOk, "expected 'want' or 'affect' line, got: '$1'", rawAffect.error)

  # Remove from shadow repo any files that are not wanted
  # TODO: remove them in reverse-alphabetical order!
  for _, path in inshadow:
    removeFile shadow.ospath path.GitFile
  # For new files, verify they are absent on disk
  for f in shadow.gitStatus:
    CHECK(f.status != '?' or not f.path.toHandler.detect, "file expected absent, but found on disk: $1", f.path)

  # Render files to their places on disk!
  for f in shadow.gitStatus:
    case f.status
    of 'M', '?':
      f.path.toHandler.affect shadow.ospath(f.path)
      shadow.git "add", "--", f.path.string
    of 'D':
      f.path.toHandler.affect shadow.ospath(f.path)
      shadow.git "rm", "--", f.path.string
    else:
      die "unexpected status $1 of file in shadow repo: $2" % [$f.status, f.path.string]

  # Finalize the deployment
  shadow.git "commit", "-m", "deployment", "--allow-empty"

proc die(msg: varargs[string, `$`]) =
  raise newException(CatchableError, msg.join "")
  # LOG "error: " & msg.join ""
  # quit 1

type
  GitRepo = distinct string  # repo path
  GitFile = distinct string  # relative path of a file in a GitRepo
  GitSubfile = distinct string
  GitStatus = tuple[status: char, path: GitFile]

proc gitFiles(repo: GitRepo, args: varargs[string]): seq[GitFile] =
  repo.rawGitLines(args).map(checkGitFile)  # FIXME: [LATER]: use gitZStrings

# FIXME: add also a command gitZStrings for NULL-separated strings
# TODO: [LATER]: write an iterator variant of this proc
proc rawGitLines(repo: GitRepo, args: varargs[string]): seq[TaintedString] =
  LOG "# cd " & repo.string & "; git " & args.join " "
  var p = startProcess("git", workingDir=repo.string, args=args, options={poUsePath})
  # echo repo.string, " git ", args
  var outp = outputStream(p)
  close inputStream(p)
  # TODO: what about errorStream(p) ?
  while not outp.atEnd:
    # FIXME: implement better readLine
    if (let l = outp.readLine(); l.len > 0):
      LOG "##" & l.string
      result.add l
  while p.peekExitCode() == -1:
    continue
  close(p)
  # TODO: [LATER]: throw exception instead of dying
  CHECK(p.peekExitCode() == 0, "command 'git $1' returned non-zero exit code: $2", args.join " ", $p.peekExitCode())

proc git(repo: GitRepo, args: varargs[string]) =
  discard repo.rawGitLines(args)

proc `[]`[T, U](s: TaintedString, x: HSlice[T, U]): TaintedString =
  s.string[x].TaintedString

proc gitStatus(repo: GitRepo): seq[GitStatus] =
  ## gitStatus returns an an array of files in the shadow repository that have
  ## differences between the working tree and the staging area ("index file"),
  ## including untracked files. Entries in the returned list have the following
  ## fields:
  ##  - one-letter 'status': M[odified] / A[dded] / D[eleted] / ? [untracked]
  ##  - relative 'path' (slash-separated)
  ##
  ## TODO: support whitespace and other nonprintable characters without error
  # FIXME: don't use die in this func, raise exceptions instead
  # FIXME: read all stdout & stderr, split into lines later -- so that we can print error message when needed
  for line in repo.rawGitLines("status", "--porcelain", "-uall", "--ignored", "--no-renames"):
    if line == "": continue
    CHECK(line.len >= 4, "line from git status too short: " & line.string)
    # TODO: support space & special chars: "If a filename contains whitespace
    # or other nonprintable characters, that field will be quoted in the manner
    # of a C string literal: surrounded by ASCII double quote (34) characters,
    # and with interior special characters backslash-escaped."
    CHECK(line.string[3] != '"', "whitespace and special characters not yet supported in paths: " & line.string[3..^1])
    let info = (status: line.string[1], path: line[3..^1].checkGitFile)
    CHECK(info.status in " MAD?", "unexpected status from git in line: " & line.string)
    # if info.status notin " MAD?": die "unexpected status from git in line: " & line.string  # {' ', 'M', 'A', 'D', '?'}
    if info.status != ' ':
      result.add info

proc ospath(repo: GitRepo, path: GitFile): string =
  repo.string / path.string

proc CHECK(cond: bool, errmsg: string, args: varargs[string, string]) =
  if not cond: die(errmsg % args)

proc LOG(msg: string) =
  stdout.writeLine msg
proc LOG_ERROR(msg: string) =
  stderr.setForegroundColor(fgRed)
  stderr.writeLine "ERROR: " & msg
  stderr.resetAttributes()

proc checkGitFile(s: TaintedString): GitFile =
  CHECK(s.len > 0, "empty path")
  const supported = {'a'..'z', 'A'..'Z', '0'..'9', '_', '/', '.', '-', '@'}
  CHECK((AllChars - supported) notin s.string, "TODO: unsupported character in path: $1", s)
  proc CHECK_NO(pattern, msg: string) =
    CHECK(pattern notin "/" & s.string & "/", "$1: found $2 in path: $3", msg, pattern, s)
  # FIXME: also sanitize other Windows-specific stuff, like 'nul' or 'con' in filenames
  # FIXME: properly handle case-insensitive filesystems (Windows, Mac?)
  CHECK_NO("//", "duplicate slash, or leading/trailing slash")
  CHECK_NO("/../", "relative path")
  CHECK_NO("/./", "denormalized path")
  return s.GitFile

proc startHandler(command: string, args: openArray[string]): Handler =
  # TODO: use poDaemon option on Windows?
  let p = startProcess(command=command, args=args, options={poUsePath, poStdErrToStdOut})
  p.inputStream.writeLine "com.akavel.mana.v1.rq"
  p.inputStream.flush
  let rs = p.outputStream.readLine
  if rs != "com.akavel.mana.v1.rs":
    LOG_ERROR "expected handshake 'com.akavel.mana.v1.rs' from handler '$1', got:\n$2" % [command, rs.string]
    p.inputStream.close
    while not p.outputStream.atEnd:
      LOG p.outputStream.readLine.string
    quit(1)
  return p.Handler

proc urldecode(s: TaintedString): TaintedString =
  decodeUrl(s.string, decodePlus=false).TaintedString
proc urlencode[T](s: T): string =
  encodeUrl(s.string, usePlus=false)

type
  Handler = distinct Process
  PathHandler = tuple[h: Handler, p: GitSubfile]

proc `<<`(ph: PathHandler, args: openArray[string]): seq[TaintedString] =
  let query = args.map(urlencode).join(" ")
  LOG query
  let h = ph.h.Process
  h.inputStream.writeLine query
  h.inputStream.flush
  let rs = h.outputStream.readLine.split " "
  var ok = rs.len >= args.len and rs[0] == args[0] & "ed"
  var i = 1
  while ok and i < args.len:
    if rs[i].urldecode.string != args[i]:
      ok = false
    inc(i)
  if not ok:
    LOG_ERROR "expected response to '$1' from handler, got:\n$2" % [query, string(rs.join " ")]
    h.inputStream.close
    while not h.outputStream.atEnd:
      LOG h.outputStream.readLine.string
    quit(1)
  return rs

proc detect(ph: PathHandler): bool =
  let rs = ph << ["detect", ph.p.string]
  CHECK(rs.len == 3, "bad result in response to 'detect $1': $2", ph.p, rs.join " ")
  case rs[2].string
  of "present": return true
  of "absent": return false
  else: die "bad result in response to 'detect $1': $2" % [ph.p.string, string(rs.join " ")]

proc gather_to(ph: PathHandler, ospath: string) =
  discard ph << ["gather", ph.p.string, ospath]

proc affect(ph: PathHandler, ospath: string) =
  discard ph << ["affect", ph.p.string, ospath]

type Match = Result[seq[string], string]

template `=~`(s: string, pattern: untyped): Match =
  # If s exactly matches npeg pattern, returns captures as success;
  # otherwise, returns s with trimmed EOL as failure"""
  let
    v = patt(pattern)
    m = v.match(s)
  if m.ok and m.matchLen == s.len:
    Match.ok m.captures
  else:
    Match.err s.dup(stripLineEnd)

