local posixfs = {}

local arg = arg
_G.arg = nil

function posixfs.forroot(root)
  local root = root:gsub('/*$', '/')
  function posixfs.exists(path)
    return posixfs.osexists(root .. path)
  end
  function posixfs.query(path, shadowpath)
    posixfs.osquery(root .. path, shadowpath)
  end
  function posixfs.apply(path, shadowpath)
    posixfs.osapply(root .. path, shadowpath)
  end
end

function posixfs.osexists(ospath)
  local fh, err = io.open(ospath, 'r')
  if fh then
    fh:close()
  end
  return not not fh
end

local function execf(cmdf, ...)
  assert(os.execute(cmdf:format(...)))
end

function posixfs.osquery(ospath, shadowpath)
  local attr = posixfs.stat(ospath)
  execf("echo '%s%s' | cat - '%s' > '%s'", 
    attr.kind, attr.mode, ospath, shadowpath)
end

function posixfs.stat(ospath)
  local h = assert(io.popen("stat -c '%f\n%a' '"..ospath.."'", "r"))
  local kind = assert(h:read'*l')
  local mode = assert(h:read'*l')
  h:close()
  if kind == "directory" then
    return {kind='d', mode=mode}
  elseif kind == "regular file" then
    return {kind='f', mode=mode}
  else
    error(("unsupported kind %q of file %q"):format(kind, ospath))
  end
end

function posixfs.osapply(ospath, shadowpath)
  if not posixfs.osexists(shadowpath) then
    assert(os.remove(ospath))
    return
  end
  local attr = posixfs.header(shadowpath)
  -- TODO[LATER]: handle changes of kind (file <-> directory)
  if attr.kind == 'd' then
    execf("mkdir '%s'", ospath)
  elseif attr.kind == 'f' then
    execf("sed '2,$!d' < '%s' > '%s'", shadowpath, ospath)
  else
    error(("unsupported file kind %q on %q"):format(attr.kind, shadowpath))
  end
  execf("chmod %s '%s'", attr.mode, ospath)
end

function posixfs.header(shadowpath)
  local fh = assert(io.open(shadowpath, 'r'))
  local header = assert(fh:read'*l')
  fh:close()
  return {kind=header:sub(1,1), mode=header:sub(2)}
end

if arg then
  require 'manaprotocol'.handle(posixfs.forroot(arg[1] or '/'))
end

return posixfs

