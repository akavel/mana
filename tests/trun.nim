{.experimental: "codeReordering".}
import unittest
import mana

test "minimal":
  testInput = mockInput"""
com.akavel.mana.v1
shadow /tmp/foo/bar
handle foobar foohandler
affect
"""
  testGit = mockGit(
    ("status", @[]))
  run(testInput, testGit)
  # TODO: verify nothing left in testInput, testGit, testHandlers


proc mockInput(s: string): proc(): string =
  let lines = s.splitLines
  var i = 0
  return proc(): string =
    result = lines[i]
    inc i

type MockGitCall = tuple[arg0: string, stdout: seq[string]]
proc mockGit(calls: varargs[MockGitCall]): gitCaller =
  var i = 0
  return proc(repo: string, args: varargs[string]): seq[string] =
    if i >= calls.len:
      raise newException(ValueError, "expected no more git calls, got `git $1`" % args.join" ")
    if args[0] != calls[i].arg0:
      raise newException(ValueError, "expected `git $1` call, got `git $2`" % [calls[i].arg0, args.join " "])
    result = calls[i].stdout
    inc i

proc mockHandlerOpener

