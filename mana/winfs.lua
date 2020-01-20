local winfs = {}

function winfs.fordisk(letter)
  if not string.match(letter, '^[a-zA-Z]$') then
    error(('disk must be one-letter, got: %q'):format(letter))
  end
  return {
    exists = function(path)
      return winfs.osexists(winfs.ospath(letter..'/'..path))
    end,
    query = function(path, shadowpath)
      winfs.copy(winfs.ospath(letter..'/'..path), shadowpath)
    end,
    apply = function(path, shadowpath)
      winfs.osapply(winfs.ospath(letter..'/'..path), shadowpath)
    end,
  }
end

-- winfs.ospath is a helper which translates a git relative path to absolute
-- path in Windows
function winfs.ospath(path)
  if not string.match(path, '^[a-zA-Z]/') then
    error(('path not valid for Windows, must be "<DISK>/<RELPATH>", got: %q'):format(path))
  end
  return path:sub(1,1) .. ":\\" .. path:sub(3):gsub("/", "\\")
end

function winfs.osexists(ospath)
  local fh, err = io.open(ospath, 'r')
  if fh then
    fh:close()
  end
  return not not fh
end

-- winfs.osapply is a helper for applying files which can be
-- reused by other Windows file-based mana plugins.
function winfs.osapply(ospath, shadowpath)
  if winfs.osexists(shadowpath) then
    winfs.mkdirp(ospath)
    winfs.copy(shadowpath, ospath)
  else
    assert(os.remove(ospath))
    -- TODO: remove parent directories if empty
  end
end

-- winfs.copy is a helper copying a file on disk from p1 to p2
function winfs.copy(p1, p2)
  -- TODO: use rsync if possible
  local cmd = "copy /b /y " .. p1 .. " " .. p2 .. " >nul"
  io.stderr:write('# ' .. cmd .. '\n')
  assert(os.execute(cmd))
end

-- winfs.mkdirp is a helper function which creates all parent directories
-- leading to the specified Windows file path. This is roughly equivalent to
-- the following Linux command:
-- 
--   $ mkdir -p "$(dirname "$ospath")"
function winfs.mkdirp(ospath)
  local iter = ospath:gmatch "([^\\]+)\\"
  local parent = iter()  -- first segment - "C:" or similar
  for d in iter do
    parent = parent .. '\\' .. d
    -- TODO: make it silent when directory already exists
    os.execute("mkdir " .. parent)
  end
end

if arg then
    require 'manaprotocol'.handle(winfs.fordisk(arg[1]))
end

return winfs

