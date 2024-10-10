local posixfiles = {}

local arg = arg
_G.arg = nil

function posixfiles.forroot(root)
  local root = root:gsub('/*$', '/')
  return {
    exists = function(path)
      return posixfiles.osexists(root .. path)
    end,
    query = function(path, shadowpath)
      posixfiles.osquery(root .. path, shadowpath)
    end,
    apply = function(path, shadowpath)
      posixfiles.osapply(root .. path, shadowpath)
    end,
  }
end

function posixfiles.osexists(ospath)
  local fh, err = io.open(ospath, 'r')
  if fh then
    fh:close()
  end
  return not not fh
end

local function execf(cmdf, ...)
  assert(os.execute(cmdf:format(...)))
end

function posixfiles.osquery(ospath, shadowpath)
  local mode = posixfiles.stat(ospath, 'regular file')
  execf("echo '%s' | cat - '%s' > '%s'", mode, ospath, shadowpath)
end

function posixfiles.stat(ospath, wantKind)
  local h = assert(io.popen("stat -c '%F\n%a' '"..ospath.."'", "r"))
  local kind = assert(h:read'*l')
  if kind ~= wantKind then
    error(("unsupported kind %q of %q (wanted %q)"):format(kind, ospath, wantKind))
  end
  local mode = assert(h:read'*l')
  h:close()
  return mode
end

function posixfiles.osapply(ospath, shadowpath)
  if not posixfiles.osexists(shadowpath) then
    assert(os.remove(ospath))
    return
  end
  local mode = posixfiles.header(shadowpath)
  execf("sed '2,$!d' < '%s' > '%s'", shadowpath, ospath)
  execf("chmod %s '%s'", mode, ospath)
end

function posixfiles.header(shadowpath)
  local fh = assert(io.open(shadowpath, 'r'))
  local header = assert(fh:read'*l')
  fh:close()
  return header
end

if arg then
  require 'manaprotocol'.handle(posixfiles.forroot(arg[1]))
end

return posixfiles

