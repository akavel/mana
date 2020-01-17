{.experimental: "codeReordering".}
import "osproc"
import "streams"
import "strutils"
import "tables"
import "uri"

# Input protocol: (eggex specification - see: https://www.oilshell.org/release/0.7.pre5/doc/eggex.html)
#
#  HANDSHAKE = / 'com.akavel.mana.v1' CR? LF /
#  SHADOW = / 'shadow ' < URLENCODED = path > CR? LF /
#  HANDLER = / 'handle ' < [a-z 0-9 _]+ = prefix > (' ' < URLENCODED = command_part >)+ CR? LF /
#   LINES = / (' ' < .* = line > CR? LF)* /
#   FILE_WITH_LINES = / ' with ' CR? LF LINES /
#   FILE_ABSENT = / ' absent ' CR? LF /
#  FILE = / 'want ' < URLENCODED = path > (FILE_WITH_LINES | FILE_ABSENT) /
#  EXEC = / 'exec' CR? LF /
#
#  INPUT = / HANDSHAKE SHADOW HANDLER+ FILE* EXEC /

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

  # handshake
  var rawline = readLine()
  const handshake = "com.akavel.mana.v1"
  if rawline != handshake: die "bad first line, expected ", handshake, ", got: '", rawline, "'"

  # shadow repo path
  var line = readLine().split ' '
  if len(line) != 2: die "bad shadow line, expected 'shadow ' and urlencoded path, got: '", line.join " ", "'"
  let shadow = decodeUrl(line[1], decodePlus=false)

  # handlers
  var handlers: Table[string, Process]
  while true:
    line = readLine().split ' '
    if len(line) == 0: die "unexpected empty line"
    if line[0] == "handle":
      if len(line) < 3: die "line missing prefix or command: '", line.join " ", "'"
      if line[1].contains(AllChars - {'a'..'z', '0'..'9', '_'}): die "only [a-z0-9_] allowed in prefix, got: '", line[1], "'"
      var
        prefix = line[1]
        command = decodeUrl(line[2], decodePlus=false)
        args: seq[string]
      for i in 3 ..< len(line):
        args.add decodeUrl(line[i], decodePlus=false)
      # TODO: use poDaemon option on Windows
      handlers[prefix] = startProcess(command=command, args=args, options={poUsePath})
      handlers[prefix].inputStream.writeLine "com.akavel.mana.v1.rq"
      let rs = handlers[prefix].outputStream.readLine
      const rsHandshake = "com.akavel.mana.v1.rs"
      if rs != rsHandshake: die "expected handshake '", rsHandshake, "' from handler '", command, "', got: '", rs, "'"
    else:
      unread = line.join " "
      break

  # wanted files
  var wanted: Table[string, string]
  var absent: Set[string]
  while true:
    line = readLine().split ' '
    if len(line) == 0: die "expected 'want' line or 'exec' line, got empty line"
    case line[0]
    of "want":
      if len(line) != 3: die "expected urlencoded path and 'with'/'absent' after 'want' in: '", line.join " ", "'"
      let path = decodeUrl(line[1], decodePlus=false)
      # FIXME: verify there are no unsafe for Windows, Linux, Mac, chars or strings in path
      # FIXME: verify there are no ".." etc. fragments in path
      case line[2]
      of "absent": absent.incl(path)
      of "with":
        var lines: seq[string]
        ...



proc die(msg: varargs[string, `$`]) =
  stderr.writeLine "error: " & msg.join ""
  quit 1

when isMainModule:
  main()
