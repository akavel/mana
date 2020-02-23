local posixdirs = {}

local arg = arg
_G.arg = nil

local posixfiles = require 'handler.posixfiles'

function posixdirs.forroot(root)
  local root = root:gsub('/*$', '/')
  return {
    exists = function(path)
      return posixfiles.osexists(root .. path)
    end,
    query = function(path, shadowpath)
      posixdirs.osquery(root .. path, shadowpath)
    end,
    apply = function(path, shadowpath)
      posixdirs.osapply(root .. path, shadowpath)
    end,
  }
end

function posixdirs.osquery(ospath, shadowpath)
  local mode = posixfiles.stat(ospath, 'directory')
  local fh = assert(io.open(shadowpath, 'w'))
  fh:write(mode..'\n')
  fh:close()
end

function posixdirs.osapply(ospath, shadowpath)
  if not posixfiles.osexists(shadowpath) then
    assert(os.remove(ospath))
    return
  end
  local mode = posixfiles.header(shadowpath)
  assert(os.execute("mkdir '"..ospath.."'"))
  assert(os.execute("chmod "..mode.." '"..ospath.."'"))
end

if arg then
  require 'manaprotocol'.handle(posixdirs.forroot(arg[1]))
end

return posixdirs

