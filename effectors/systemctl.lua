local systemctl = {}

local arg = arg
_G.arg = nil

local posixfs = require 'effectors.posixfs'

local function exec(cmd)
  local p = io.popen(cmd, 'r')
  local d = p:read '*a'
  p:close()
  local function trim(s)
    return (s:match '^%s*(.-)%s*$')
  end
  return trim(d)
end

function systemctl.exists(path)
  return exec('systemctl is-active ' .. path) == 'active' and
    exec('systemctl is-enabled ' .. path) == 'enabled'
end
function systemctl.query(path, shadowpath)
  systemctl.touch(shadowpath)
end
function systemctl.apply(path, shadowpath)
  if posixfs.osexists(shadowpath) then
    os.execute('systemctl -q enable --now ' .. path)
  else
    os.execute('systemctl -q disable --now ' .. path)
  end
end

function systemctl.touch(ospath)
  local fh = assert(io.open(ospath, 'a'))
  fh:close()
end

return systemctl

