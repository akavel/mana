local winhome = {}

local winfs = require 'nnn.winfs'

local root = 'home/'
local function trim_root(path)
	return path:sub(#root+1)
end

function winhome.owns(path)
	return not not path:find('^' .. root)
end

function winhome.exists(path)
	return winfs.osexists(winhome.ospath(path))
end
function winhome.query(path, shadowf)
	winfs.copy(winhome.ospath(path), shadowf(path))
end
function winhome.apply(path, shadowf)
	winfs.osapply(winhome.ospath(path), shadowf(path))
end

function winhome.ospath(path)
	return os.getenv('userprofile') .. '\\' .. trim_root(path):gsub('/', '\\')
end

-- io.stderr:write('[' .. winhome.ospath("home/.vim/vimrc") .. ']')

return winhome

