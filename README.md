# T1-Customizer

Free configuration tool for the **T1 mini keyboard** — the
[6 Keys 1 Knob RGB Programming Macro Gaming Keypad](https://ja.aliexpress.com/item/1005009812219099.html) (AliExpress,
USB VID `1189` / PID `8890`).

> Forked from T1-VREmoKey (えもかえ) by AppWizz Kai, version 2.2.4.
> This fork removes every VRChat-related feature and the activation
> (license) gate of the original. It is free, and every feature is
> available to everyone.

English | [日本語](README.ja.md) | [简体中文](README.zh-CN.md) | [한국어](README.ko.md)

## What it does

- Key assignments (up to 5 modifier-combination groups per key)
- App presets: one click writes Adobe Premiere / Illustrator / Photoshop 2026 and CLIP STUDIO PAINT shortcut sets to KEY1–KEY6 (see `docs/app-presets.md`)
- Named modifier combinations, multimedia keys, mouse actions, LED modes
- Preset 1/2/3 switching and flashing to the keyboard
- Dial rotation → OS volume (microphone / headphone), dial press switches
  the mode; the listener stays in the tray
- Knob press choice: on the Dial page, pick whether the knob switches
  the volume mode (default) or acts as keys — in key mode, all three
  knob patterns (press K1-C / K2-C, rotate left, rotate right) can be
  selected on the key panel and written like any other key
- CLIP STUDIO PAINT preset knob rotation: in key mode, applying the
  preset also writes canvas Rotate Left / Right to K1-L / K1-R
- `settings.json` is portable across Linux / Windows / macOS (all codes
  stored as Linux evdev codes). Settings written by the original app for
  these features load unchanged; settings for removed features are
  ignored and kept in the file.
- Window language: Japanese, English, or Korean, saved as `lang` in
  `settings.json`

## What is not in this fork

- No VRChat support of any kind: no OSC, no avatar profiles, no
  expression hotkeys, no Unity package.
- No activation, license, trial, or order-number gate. There is nothing
  to unlock.
- Rust only. The older Python implementation is not included.

## Status

| Target OS | Status |
| --- | --- |
| Linux | Implemented; not yet confirmed on a physical T1 in this fork |
| Windows 11 | Implemented; not yet confirmed on a physical T1 in this fork |
| macOS | Implemented, **not verified** (no macOS machine in this environment) |

The hardware protocol code is unchanged from the original 2.2.4, which
was itself not yet confirmed on a physical T1 (the Python 1.4.0
implementation was).

## Usage

```bash
t1-keyboard-config            # configuration GUI
t1-keyboard-config --selftest # device, permission and write-path diagnostics
t1-keyboard-config --watch    # live stream of T1 input events
t1-keyboard-config --monitor  # resident dial volume listener
t1-keyboard-config --version
```

Closing the window leaves the process in the tray
(Open window / start or stop the resident listener / Quit).

### Verify that writes take effect

```bash
t1-keyboard-config --watch    # 1) press KEY1 to see its current code
```

In the GUI select `KEY1`, open the `Multimedia` tab, pick "Play/Pause" and
press **Write**. Then run `--watch` again and press KEY1; it should now
report `KEY_PLAYPAUSE` (write "None" to restore the original).

## Build from source

```bash
cd rust
cargo build --release
cargo test
```

Linux GUI builds need the usual GTK 3 development packages
(`libgtk-3-dev`, `libayatana-appindicator3-dev`).

## Requirements

| Item | Linux | Windows 11 | macOS |
| --- | --- | --- | --- |
| Runtime | the Rust binary | the Rust binary | the Rust binary |
| GUI | egui (built in) | egui (built in) | egui (built in) |
| Write path | usbfs (`/dev/bus/usb`) | HID (`WriteFile`) | IOHID (`SetReport`) |
| Volume control | `wpctl` (optional) | Core Audio, then key events | `osascript` (built in) |
| Extra packages | `pipewire-utils` for dial volume | none | none |

On Linux, install `99-t1-keyboard.rules` into
`/etc/udev/rules.d/` so the device nodes are accessible without root.

Japanese in the window uses a system face when one is installed, and
otherwise a subset of Noto Sans JP (SIL Open Font License 1.1,
© 2014-2021 Adobe, Reserved Font Name "Source"). The license text is
`rust/assets/fonts/OFL.txt` and on the Help tab. Korean (Hangul) is drawn
with a bundled Noto Sans KR subset under the same license.

## License

MIT. See `LICENSE`.
