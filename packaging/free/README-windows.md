# T1-Customizer (Windows)

Free configuration tool for the T1 mini keyboard (VID 1189 / PID 8890).

## Install

Run `t1-customizer-<version>-setup.exe`, or use the portable binaries
directly: `t1-keyboard-config-gui.exe` (windowed app) and
`t1-keyboard-config.exe` (console).

The binaries are unsigned: Windows may show a SmartScreen warning.
Click "More info" -> "Run anyway" to proceed.

## Run

- GUI: `t1-keyboard-config-gui.exe`
- Diagnostics: `t1-keyboard-config.exe --selftest`
- Resident dial volume listener: `t1-keyboard-config.exe --monitor`

Closing the window leaves the app in the tray.
