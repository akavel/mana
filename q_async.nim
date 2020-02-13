# {.experimental: "codeReordering".}
import os
import asynctools/[asyncproc, asyncpipe]
import asyncfutures
import asyncdispatch

var p = startProcess("q_outerr", options = {})
var pout = p.outputHandle
var perr = p.errorHandle

proc next() =
  var bufo = newString(64)
  var fo = pout.readInto(bufo[0].addr, bufo.len)
  var bufe = newString(64)
  var fe = perr.readInto(bufe[0].addr, bufe.len)
  var fx = p.waitForExit()
  while true:
    waitFor(fe or fo or fx)
    if fo.finished:
      if fo.read > 0:
        echo "O ", fo.read(), " ", bufo.substr(0, fo.read()-1)
      fo = pout.readInto(bufo[0].addr, bufo.len)
    if fe.finished:
      if fe.read > 0:
        echo "E ", fe.read(), " ", bufe.substr(0, fe.read()-1)
      fe = perr.readInto(bufe[0].addr, bufe.len)
    if fx.finished:
      echo "X ", fx.read()
      return

next()
