{.experimental: "codeReordering".}
import asynctools/[asyncproc, asyncpipe]
import asyncfutures
import asyncdispatch

# var p = startProcess("q_outerr", options = {poInteractive})
# var p = startProcess("q_outerr", options = {}, pipeStdout = pout, pipeStderr = perr)

proc next() {.async.} =
# proc next() =
  var p = startProcess("q_outerr", options = {})
  var pout = p.outputHandle
  var perr = p.errorHandle

  var bufo = newString(64)
  let fo = pout.readInto(bufo[0].addr, bufo.len)
  # let fo = readInto(pout, bufo[0].addr, bufo.len)
  var bufe = newString(64)
  let fe = perr.readInto(bufe[0].addr, bufe.len)
  # await (fo or fe)
  await fo or fe
  if fo.finished:
    echo "O ", fo.read(), " ", bufo.substr(0, fo.read()-1)
  elif fe.finished:
    echo "E ", fe.read(), " ", bufe.substr(0, fe.read()-1)

waitFor next()
