local mana = assert(io.popen('./mana', 'w'))
mana:write [[
com.akavel.mana.v1
shadow /home/pi/prog/shadow
handle homed lua handler/posixdirs.lua /home/pi
handle homef lua handler/posixfiles.lua /home/pi
]]

---------------------------------------------------------

function want(path)
  return function(contents)
    mana:write("want " .. path .. "\n")
    for line in contents:gmatch "[^\n]*" do
      mana:write(" " .. line .. "\n")
    end
  end
end

want "homed/.ssh" "700"
want "homef/.ssh/authorized_keys" [[
600
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAuLKyzgasGVcn3OkEoF0YF2X4fcQoWdNsLT/dwnY1Ym rpi4-sf7-git-ed25519
]]

---------------------------------------------------------

mana:write [[
affect
]]
