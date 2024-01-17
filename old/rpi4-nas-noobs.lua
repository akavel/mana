local mana = assert(io.popen('mana', 'w'))
mana:write [[
com.akavel.mana.v1
shadow c:\prog\shadow-rpi4-nas-noobs
handle boot lua53 handler/winfs.lua 4a8bd31d-4939-11ea-96ee-4074e03cda01
]]

-- TODO: authorized_keys: ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAuLKyzgasGVcn3OkEoF0YF2X4fcQoWdNsLT/dwnY1Ym rpi4-sf7-git-ed25519

function want(path)
  return function(contents)
    mana:write("want " .. path .. "\n")
    for line in contents:gmatch "[^\n]*" do
      mana:write(" " .. line .. "\n")
    end
  end
end

--[[
lua53 -e "for i=2,20 do print(i) local p=assert(io.popen(('ping 192.168.0.%d -n 1'):format(i))) local d=p:read'*a' p:close() if not d:match'Destination host unreachable' then print(d) end end"
2

--> 192.168.0.16 on my net as of now
]]

-- Enable SSH server in NOOBS
-- https://raspberrypi.stackexchange.com/a/67353
want "boot/ssh" ""

-- Original, for now.
-- See also: https://raspberrypi.stackexchange.com/a/59661
want "boot/cmdline.txt" [[
console=serial0,115200 console=tty1 root=PARTUUID=093bedcc-02 rootfstype=ext4 elevator=deadline fsck.repair=yes rootwait quiet init=/usr/lib/raspi-config/init_resize.sh splash plymouth.ignore-serial-consoles
]]

-- WiFi config
--   ssid="..."
--   psk="..."
want "boot/wpa_supplicant.conf" ([[
ctrl_interface=DIR=/var/run/wpa_supplicant GROUP=netdev
ap_scan=1
update_config=1
country=PL

network={
]] .. require "_wifi" .. [[
}
]])

mana:write [[
affect
]]
