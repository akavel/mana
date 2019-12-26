local winpath = {}

local winfs = require 'nnn.winfs'

local root = 'path/user/'
local function trim_root(path)
	return path:sub(#root+1)
end
-- simplify path for comparison purposes
local function simplify(ospath)
	local ospath = ospath:gsub('\\+$', '')  -- trim backslash
	return ospath:lower()
end
local function powershell(cmd)
	return 'powershell -command " ' .. cmd .. ' "'
end

function winpath.owns(path)
	return path:find('^' .. root)
end

function winpath.exists(path)
	local ospath = simplify(winfs.ospath(trim_root(path)))
	for p in winpath.iter() do
		if simplify(p) == ospath then
			return true
		end
	end
	return false
end

function winpath.query(path, shadowf)
	winpath.touch(shadowf(path))
end

function winpath.apply(path, shadowf)
	local adding = winfs.osexists(shadowf(path))
	local ospath = winfs.ospath(trim_root(path))
	local newpath = {}
	local found = false
	for p in winpath.iter() do
		if simplify(p) == simplify(ospath) then
			if adding then
				return  -- already there, no need to add
			else
				-- don't put in newpath
			end
		else
			newpath[#newpath+1] = p
			if p:find '"' or p:find "'" then
				error(("old PATH contains quote, currently cannot be handled properly: %q"):format(p))
			end
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

function winpath.touch(ospath)
	local fh = assert(io.open(ospath, 'a'))
	fh:close()
end

-- for p in winpath.iter() do
-- 	io.stderr:write('['..p..']\n')
-- end

return winpath

