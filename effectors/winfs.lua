local winfs = {}

local arg = arg
_G.arg = nil

function winfs.init(uuid)
  return winfs.fordisk(uuid)
end

-- winfs.fordisk returns an effector for affecting files on a Windows disk
-- (partition) identified by a UUID. To get UUIDs of currently connected disks
-- on Windows, run `mountvol` command.
function winfs.fordisk(uuid)
  local uuids = winfs.diskUUIDs()
  local letter = uuids[uuid:lower()]
  if not letter or letter == '' then
    error(('disk for uuid %q not found; run `mountvol` to see what is available'):format(uuid))
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
  --io.stderr:write('# ' .. cmd .. '\n')
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
    os.execute("mkdir " .. parent .. " 2>nul >nul")
  end
end

function winfs.diskUUIDs()
  -- Fragment of example output to parse:
  --[[
  Possible values for VolumeName along with current mount points are:

      \\?\Volume{39b9d89e-9522-45a3-a12b-450f027d0bf0}\
          C:\

      \\?\Volume{1e143f42-c648-498d-91a4-d86f91cbc714}\
          *** NO MOUNT POINTS ***

      \\?\Volume{94f8509a-079a-44b0-91f4-85769f3e2fea}\
          *** NO MOUNT POINTS ***
  ]]
  local pUuid = '^%s*\\\\%?\\Volume{(.*)}\\%s*$'
  local pDisk = '([A-Z]):\\'
  local pNoDisk = '*** NO MOUNT POINTS ***'

  local map = {}

  local p = assert(io.popen('mountvol', 'r'))
  while true do
    local line = p:read '*l'
    if not line then break end
    local uuid = line:match(pUuid)
    if uuid then
      local mount = p:read('*l'):gsub('^%s*',''):gsub('%s*$','')
      local disk = mount:match('^'..pDisk..'$')
      if mount == pNoDisk then
        map[uuid:lower()] = ''
      elseif disk then
        map[uuid:lower()] = disk
      else
        error(('unexpected format of mount point for UUID %s: %s'):format(uuid, mount))
      end
    end
  end
  p:close()
  return map
end

return winfs

