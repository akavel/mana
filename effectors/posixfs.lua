local posixfs = {}

local arg = arg
_G.arg = nil

function posixfs.exists(path)
  return posixfs.osexists(posixfs.ospath(path))
end
function posixfs.query(path, shadowpath)
  posixfs.copy(posixfs.ospath(path), shadowpath)
end
function posixfs.apply(path, shadowpath)
  posixfs.osapply(posixfs.ospath(path), shadowpath)
end

function posixfs.ospath(path)
  return '/' .. path
end

function posixfs.osexists(ospath)
  local fh, err = io.open(ospath, 'r')
  if fh then
    fh:close()
  end
  return not not fh
end

function posixfs.copy(p1, p2)
  -- TODO: use rsync if possible
  assert(os.execute("cp '" .. p1 .. "' '" .. p2 .. "'"))
end

function posixfs.osapply(ospath, shadowpath)
  if posixfs.osexists(shadowpath) then
    posixfs.mkdirp(ospath)
    posixfs.copy(shadowpath, ospath)
  else
    assert(os.remove(ospath))
    -- TODO: remove parent directories if empty
  end
end

function posixfs.mkdirp(ospath)
  local parent = ospath:gsub('/[^/]*$', '')
  os.execute("mkdir -p '" .. parent .. "'")
end

return posixfs

