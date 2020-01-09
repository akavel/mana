local v1 = {}

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
    elseif l == "com.akavel.mana.v1.rq" then
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
    io.write 'com.akavel.mana.v1.rs\n'
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

function v1.parse(handler)
    while true do
        local cmd = v1.read()
        if not cmd then
            return
        end
        local resp = handler[cmd.cmd](cmd.path, cmd.shadowpath)
        v1.write(resp)
    end
end

