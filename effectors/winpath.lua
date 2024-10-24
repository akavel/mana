-- Package winpath handles setting of User's PATH variable in registry.
-- NOTE: Handling Machine-wide PATH is currently unsupported, as it
-- requires elevated (i.e. Administrator) privileges.
local winpath = {}

local arg = arg
_G.arg = nil

local winfs = require 'effectors.winfs'

-- simplify path for comparison purposes
local function simplify(ospath)
  local ospath = ospath:gsub('\\+$', '')  -- trim backslash
  return ospath:lower()
end
local function powershell(cmd)
  return 'powershell -command " ' .. cmd .. ' "'
end

function winpath.init()
  return winpath
end

function winpath.exists(path)
  local ospath = simplify(winfs.ospath(path))
  for p in winpath.iter() do
    if simplify(p) == ospath then
      return true
    end
  end
  return false
end

function winpath.query(path, shadowpath)
  winpath.mkempty(shadowpath)
end

function winpath.apply(path, shadowpath)
  local adding = winfs.osexists(shadowpath)
  local ospath = winfs.ospath(path)
  local newpath = {}
  local found = false
  for p in winpath.iter() do
    if simplify(p) ~= simplify(ospath) then
      newpath[#newpath+1] = p
      if p:find '"' or p:find "'" then
        error(("old PATH contains quote, currently cannot be handled properly: %q"):format(p))
      end
    elseif adding then
      return  -- already there, no need to add
    end
  end
  if adding then
    newpath[#newpath+1] = ospath
  end
  local cmd = "[System.Environment]::SetEnvironmentVariable('PATH','" .. table.concat(newpath, ';') .. "',[System.EnvironmentVariableTarget]::User)"
  assert(os.execute(powershell(cmd)))
end

-- was: C:\Users\Mateusz\AppData\Local\Microsoft\WindowsApps;C:\tools\neovim\Neovim\bin;C:\Users\Mateusz\AppData\Local\Keybase\;C:\Users\Mateusz\AppData\Roaming\Programs\Zero Install

function winpath.iter()
  local pipe = assert(io.popen(powershell "[System.Environment]::GetEnvironmentVariable('PATH',[System.EnvironmentVariableTarget]::User)" ))
  local text, err = pipe:read '*a'
  if not text then
    error(err or 'empty powershell output')
  end
  text = text:gsub(';?%s*$', '')  -- right trim ';' and whitespace
  return text:gmatch '[^;]*'
end

function winpath.mkempty(ospath)
  assert(io.open(ospath, 'w')):close()
end

if arg then
  require 'manaprotocol'.handle(winpath)
end

return winpath

