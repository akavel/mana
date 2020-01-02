local winhome = {}

local winfs = require 'nnn.winfs'

function winhome.exists(path)
	return winfs.osexists(winhome.ospath(path))
end
function winhome.query(path, shadowpath)
	winfs.copy(winhome.ospath(path), shadowpath)
end
function winhome.apply(path, shadowpath)
	winfs.osapply(winhome.ospath(path), shadowpath)
end

function winhome.ospath(path)
	return os.getenv('userprofile') .. '\\' .. path:gsub('/', '\\')
end

return winhome

