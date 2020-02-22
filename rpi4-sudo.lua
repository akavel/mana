if os.getenv 'USER' ~= 'root' then
  error 'error: must be run as root'
end

local mana = assert(io.popen('./mana', 'w'))
mana:write [[
com.akavel.mana.v1
shadow /home/pi/prog/shadow-sudo
handle root lua handler/posixfs.lua
handle systemctl lua handler/systemctl.lua
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

-- Try to disable RPi disappearing from WiFi network after some time

want "root/etc/systemd/system/wifi_powersave@.service" [[
[Unit]
Description=Set WiFi power save %i
After=sys-subsystem-net-devices-wlan0.device

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/sbin/iw dev wlan0 set power_save %i

[Install]
WantedBy=sys-subsystem-net-devices-wlan0.device
]]

want "systemctl/wifi_powersave@on" ""

---------------------------------------------------------

mana:write [[
affect
]]
