# T1-Customizer (Linux)

Free configuration tool for the T1 mini keyboard (VID 1189 / PID 8890).

## Install

Copy `t1-keyboard-config` and `t1-keyboard-config-gui` onto your PATH
(for example `~/.local/bin/`), and install the udev rule so the device
is accessible without root:

```bash
sudo cp 99-t1-keyboard.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
```

## Run

```bash
t1-keyboard-config            # GUI
t1-keyboard-config --monitor  # resident dial volume listener
t1-keyboard-config --selftest # diagnostics
```

Dial volume control on Linux uses `wpctl` (package `pipewire-utils`).
