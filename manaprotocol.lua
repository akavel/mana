local v1 = {
  handshake = {
    rq = 'com.akavel.mana.v1.rq',
    rs = 'com.akavel.mana.v1.rs',
  },
}

local function hasprefix(s, prefix)
  return #s >= #prefix and s:sub(1,#prefix) == prefix
end
local function panick(line)
  error(('bad com.akavel.mana.v1.rq line format: %q'):format(line))
end
-- TODO: make word only allow urlencoded format
local word = '([^ ]+)'

function v1._urldecode(s)
  return s:gsub('%%([0-9a-fA-F][0-9a-fA-F])', function(c)
    return string.char(('0x'..c)+0)
  end)
end
function v1._urlencode(s)
  -- TODO: find out what chars are encoded in urlencoding
  return s:gsub('[^a-zA-Z.=_-]', function(c)
    return ('%%%02x'):format(string.byte(c))
  end)
end

function v1.read()
  local l = io.read '*l'
  if not l then
    return nil
  elseif l == v1.handshake.rq then
    return v1.read()
  elseif hasprefix(l, 'detect ') then
    local cmd, path, extra = l:match('^'..word..' '..word..'(.*)$')
    if extra or not cmd then
      panick(l)
    end
    return {cmd=cmd, path=v1._urldecode(path)}
  elseif hasprefix(l, 'gather ') or hasprefix(l, 'affect ') then
    local cmd, path, shadowpath, extra = l:match('^'..word..' '..word..' '..word..'(.*)$')
    if extra or not cmd then
      panick(l)
    end
    return {cmd=cmd, path=v1._urldecode(path), shadowpath=v1._urldecode(shadowpath)}
  else
    panick(l)
  end
end

function v1.write(resp)
  io.write(v1.handshake.rs .. '\n')
  if resp.cmd == 'detected' then
    if resp.result ~= 'present' and resp.result ~= 'absent' then
      error(('expected .result "present" or "absent", got: %q'):format(resp.result))
    elseif not resp.path then
      error('missing .path')
    end
    io.write(('%s %s %s\n'):format(resp.cmd, v1._urlencode(resp.path), v1._urlencode(resp.result)))
  elseif resp.cmd == 'gathered' or resp.cmd == 'affected' then
    if not resp.path or not resp.shadowpath then
      error('missing .path or .shadowpath')
    end
    io.write(('%s %s %s\n'):format(resp.cmd, v1._urlencode(resp.path), v1._urlencode(resp.shadowpath)))
  else
    error(('unexpected .cmd: %q'):format(resp.cmd))
  end
end

-- v1.handle calls v1.read in a loop and dispatches commands from
-- handler based on the input, then writes response to the output.
function v1.handle(handler)
  while true do
    local cmd = v1.read()
    if not cmd then
      return
    end
    local resp = handler[cmd.cmd](cmd.path, cmd.shadowpath)
    v1.write(resp)
  end
end

function v1.call(name, pipe, cmd, ...)
  local args = {...}
  for i, v in ipairs(args) do
    args[i] = v1._urlencode(v)
  end
  pipe:write(cmd..' '..table.concat(args, ' ')..'\n')
  local resp = pipe:read '*l'
  local want = cmd..'ed '..table.concat(args, ' '))
  if not hasprefix(resp, want) then
    error(('expected response %q but got %q from program %q'):format(
      want, resp, name))
  end
  resp = resp:sub(#want)
  if resp == '' then
    return
  elseif hasprefix(resp, ' ') then
    return resp:sub(2)
  else
    error(('expected space after %q in response %q from program %q'):format(
      want, resp, name))
  end
end

-- v1.cmd starts a shell commandline and opens a pipe channel to it. The
-- function returns an object conforming to mana plugins interface, where
-- each method call is serialized over pipe, then a response is read from
-- the pipe and returned.
function v1.cmd(commandline)
  local pipe = assert(io.popen(commandline))
  pipe:write(v1.handshake.rq)
  local handshake = pipe:read '*l'
  if handshake ~= v1.handshake.rs then
    error(('expected handshake response %q from program %q but got: %q'):format(
      v1.handshake.rs, commandline, handshake))
  end
  return {
    exists = function(path)
      local resp = v1.call(commandline, pipe, 'detect', path)
      if resp == 'present' then
        return true
      elseif resp == 'absent' then
        return false
      else
        error(('bad response %q to "detect" from %q'):format(resp, commandline))
      end
    end,
    query = function(path, shadowpath)
      v1.call(commandline, pipe, 'gather', path, shadowpath)
    end,
    apply = function(path, shadowpath)
      v1.call(commandline, pipe, 'affect', path, shadowpath)
    end,
  }
end
