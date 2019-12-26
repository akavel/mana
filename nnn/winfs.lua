local winfs = {}

function winfs.owns(path)
	return not not path:find "^[a-zA-Z]/"
end

function winfs.exists(path)
	return winfs.osexists(winfs.ospath(path))
end

-- winfs.query invokes copy command to copy file from filesystem into specified
-- path in the git repository controlled by shadowf
function winfs.query(path, shadowf)
	winfs.copy(winfs.ospath(path), shadowf(path))
end

function winfs.apply(path, shadowf)
	winfs.osapply(winfs.ospath(path), shadowf(path))
end

-- winfs.ospath is a helper which translates a git relative path to absolute
-- path in Windows
function winfs.ospath(path)
	if not winfs.owns(path) then
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
-- reused by other Windows file-based nnn plugins.
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
	local cmd = "copy /b /y " .. p1 .. " " .. p2
	io.stdout:write('# ' .. cmd .. '\n')
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

return winfs

