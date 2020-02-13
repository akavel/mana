# {.experimental: "codeReordering".}
import asynctools/[asyncproc, asyncpipe]
import asyncfutures
import asyncdispatch

# var p = startProcess("q_outerr", options = {poInteractive})
# var p = startProcess("q_outerr", options = {}, pipeStdout = pout, pipeStderr = perr)
var p = startProcess("q_outerr", options = {})
var pout = p.outputHandle
var perr = p.errorHandle

proc next() {.async.} =
# proc next() =

  var bufo = newString(1)
  var fo = pout.readInto(bufo[0].addr, bufo.len)
  # let fo = readInto(pout, bufo[0].addr, bufo.len)
  var bufe = newString(1)
  var fe = perr.readInto(bufe[0].addr, bufe.len)
  # var fx = p.waitForExit()
  # await (fo or fe)
  # proc xx(): Future[void] {.async.} =
  #   return (fo or fe)
  # await xx()
  while true:
    # echo "await"
    # await fe or fo or fx
    await fe or fo
    # echo "got"
    if fo.finished:
      if fo.read > 0:
        echo "O ", fo.read(), " ", bufo.substr(0, fo.read()-1)
      fo = pout.readInto(bufo[0].addr, bufo.len)
    if fe.finished:
      if fe.read > 0:
        echo "E ", fe.read(), " ", bufe.substr(0, fe.read()-1)
      fe = perr.readInto(bufe[0].addr, bufe.len)
    # if fx.finished:
    #   echo "X ", fx.read()
    #   return
    # echo "fin?"
    if fo.finished and fe.finished and fo.read==0 and fe.read==0:
      return

waitFor next()
# waitFor next()
