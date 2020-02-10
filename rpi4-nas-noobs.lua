local mana = assert(io.popen('mana', 'w'))
mana:write [[
com.akavel.mana.v1
shadow c:\prog\shadow-rpi4-nas-noobs
handle boot lua53 handler/winfs.lua 4a8bd31f-4939-11ea-96ee-4074e03cda01
]]

function want(path)
  return function(contents)
    mana:write("want " .. path .. "\n")
    for line in contents:gmatch "[^\n]*" do
      mana:write(" " .. line .. "\n")
    end
  end
end

-- Enable SSH server in NOOBS
-- https://raspberrypi.stackexchange.com/a/67353
want "boot/ssh" ""

-- Set static IP
-- https://raspberrypi.stackexchange.com/a/59661
want "boot/cmdline.txt" [[
dwc_otg.lpm_enable=0 console=serial0,115200 console=tty1 root=/dev/mmcblk0p7 rootfstype=ext4 elevator=deadline fsck.repair=yes rootwait ip=192.168.0.195 quiet init=/usr/lib/raspi-config/init_resize.sh splash plymouth.ignore-serial-consoles sdhci.debug_quirks2=4
]]

-- WiFi config
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
