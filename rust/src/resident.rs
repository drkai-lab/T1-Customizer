//! Resident dial listener, `--watch` event printer, and `--selftest`.
//!
//! The monitor only does OS volume work: dial rotation changes the default
//! sink ("headphone") or source ("mic") volume, and a dial press cycles
//! between the enabled modes. There is no network behaviour.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::input::{self, InputSource};
use crate::pair_chord;
use crate::preset_gesture::{self, PresetGesture};
use crate::settings::{self, Settings};
use crate::state;
use crate::tables::{self, EV_KEY, EV_REL};
use crate::volume;

static DIAL_HELD: AtomicBool = AtomicBool::new(false);

pub fn set_dial_held(held: bool) {
    DIAL_HELD.store(held, Ordering::SeqCst);
}

pub fn dial_is_held() -> bool {
    DIAL_HELD.load(Ordering::SeqCst)
}

/// Dial modes that change an OS volume. Anything else stored in
/// `dial.modes` by the original app (for example its network modes) is ignored.
pub fn dial_modes(settings: &Settings) -> Vec<String> {
    let saved = settings
        .get("dial")
        .and_then(|v| v.as_object())
        .and_then(|d| d.get("modes"))
        .and_then(|v| v.as_array());
    let mut modes: Vec<String> = saved
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str())
                .filter(|name| *name == "mic" || *name == "headphone")
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if modes.is_empty() {
        modes = vec!["mic".into(), "headphone".into()];
    }
    modes
}

/// What a dial press does, from `dial.press_action` in settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressAction {
    /// The press cycles the volume mode (the default, historic behaviour).
    Cycle,
    /// The knob is plain keys: K1-L/C/R and K2-L/C/R carry written key
    /// assignments and the monitor ignores press and rotation entirely.
    Key,
}

/// The configured press action. Anything but `"key"` means `Cycle`, so
/// settings written before this option existed behave as before.
pub fn press_action(settings: &Settings) -> PressAction {
    match settings
        .get("dial")
        .and_then(|v| v.as_object())
        .and_then(|d| d.get("press_action"))
        .and_then(|v| v.as_str())
    {
        Some("key") => PressAction::Key,
        _ => PressAction::Cycle,
    }
}

/// True while the whole knob (press and rotation) is in key assignment
/// mode: every knob slot K1-L/C/R and K2-L/C/R carries a written key
/// assignment, so the monitor must ignore the knob entirely.
pub fn knob_is_key_mode(settings: &Settings) -> bool {
    press_action(settings) == PressAction::Key
}

/// Linux pair-chord path: the grabbed T1, the decide engine, and the
/// bookkeeping that keeps the virtual device's modifier state honest.
/// Every non-sentinel event is forwarded through the virtual device
/// unchanged (knob rotation, media keys, everything) and also surfaced
/// to the monitor loop in normalised form, so the dial keeps working
/// while pair mode is on.
#[cfg(target_os = "linux")]
struct ChordPump {
    grabbed: input::GrabbedT1,
    engine: pair_chord::PairChordEngine,
    start: Instant,
    /// Physical modifier state in the T1's own event stream.
    lctrl: bool,
    lshift: bool,
    /// Modifier state currently held on the virtual device.
    virt_lctrl: bool,
    virt_lshift: bool,
    /// Sentinel F-keys currently down, by key number 1..=6. An F-key up
    /// is a sentinel release while its mark is set, even when the
    /// modifier ups arrived first.
    sentinel_down: [bool; 7],
    /// The forwarded sentinel Ctrl/Shift were already cancelled with
    /// virtual ups ahead of the first injection.
    mods_released: bool,
}

#[cfg(target_os = "linux")]
impl ChordPump {
    fn open(cfg: pair_chord::PairChordConfig) -> Option<Self> {
        let grabbed = input::GrabbedT1::open()?;
        state::log_line(&format!(
            "pair chords: monitor attached to {}",
            grabbed.describe()
        ));
        Some(Self {
            grabbed,
            engine: pair_chord::PairChordEngine::new(cfg),
            start: Instant::now(),
            lctrl: false,
            lshift: false,
            virt_lctrl: false,
            virt_lshift: false,
            sentinel_down: [false; 7],
            mods_released: false,
        })
    }

    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    fn shutdown(&mut self) {
        let actions = self.engine.shutdown();
        self.execute(actions);
    }

    fn execute(&mut self, actions: Vec<pair_chord::Action>) {
        use pair_chord::Action;
        for action in actions {
            match action {
                Action::InjectDown(combo) => {
                    if !self.mods_released && (self.virt_lctrl || self.virt_lshift) {
                        // The sentinel's own Ctrl/Shift were forwarded
                        // when the key went down; cancel them so the
                        // injected combo is seen on its own.
                        if self.virt_lctrl {
                            self.grabbed.emit_key(29, false);
                            self.virt_lctrl = false;
                        }
                        if self.virt_lshift {
                            self.grabbed.emit_key(42, false);
                            self.virt_lshift = false;
                        }
                        self.grabbed.sync();
                        self.mods_released = true;
                    }
                    for code in pair_chord::evdev_codes(combo) {
                        self.grabbed.emit_key(code, true);
                    }
                    self.grabbed.sync();
                }
                Action::InjectUp(combo) => {
                    for code in pair_chord::evdev_codes(combo).iter().rev() {
                        self.grabbed.emit_key(*code, false);
                    }
                    self.grabbed.sync();
                }
            }
        }
        if self.engine.is_idle() && !self.lctrl && !self.lshift {
            self.mods_released = false;
        }
    }

    fn handle_raw(&mut self, ev: input::RawKernelEvent, out: &mut Vec<input::InputEvent>) {
        const KERNEL_EV_KEY: u16 = 1;
        if ev.etype == KERNEL_EV_KEY {
            match ev.code {
                29 => self.lctrl = ev.value != 0,
                42 => self.lshift = ev.value != 0,
                _ => {}
            }
            if ev.value == 0 && self.mods_released && matches!(ev.code, 29 | 42) {
                // This modifier up was already delivered virtually ahead
                // of an injection; do not deliver it twice.
                if !self.lctrl && !self.lshift && self.engine.is_idle() {
                    self.mods_released = false;
                }
                return;
            }
            if (59..=64).contains(&ev.code) {
                let keynum = (ev.code - 58) as usize;
                match ev.value {
                    1 if self.sentinel_down[keynum] => return, // duplicate
                    1 if self.lctrl && self.lshift => {
                        self.sentinel_down[keynum] = true;
                        let actions = self.engine.key_down(keynum as u8, self.now());
                        self.execute(actions);
                        return;
                    }
                    0 if self.sentinel_down[keynum] => {
                        self.sentinel_down[keynum] = false;
                        let actions = self.engine.key_up(keynum as u8, self.now());
                        self.execute(actions);
                        return;
                    }
                    2 if self.sentinel_down[keynum] => return, // repeat
                    _ => {}
                }
            }
        }
        self.grabbed.emit(ev.etype, ev.code, ev.value);
        match ev.code {
            29 if ev.etype == KERNEL_EV_KEY => self.virt_lctrl = ev.value != 0,
            42 if ev.etype == KERNEL_EV_KEY => self.virt_lshift = ev.value != 0,
            _ => {}
        }
        if let Some(etype) = tables::normalize_kernel_type(ev.etype) {
            out.push(input::InputEvent {
                etype,
                code: ev.code,
                value: ev.value,
            });
        }
    }

    /// Poll the grabbed devices, run the engine, and return the events
    /// the monitor loop should still see (everything but sentinels).
    fn pump(&mut self, max_timeout: f64) -> Vec<input::InputEvent> {
        let timeout = match self.engine.next_deadline() {
            Some(deadline) => {
                (deadline.saturating_sub(self.now()) as f64 / 1000.0).clamp(0.0, max_timeout)
            }
            None => max_timeout,
        };
        let raw = self.grabbed.poll(timeout);
        let mut out = Vec::new();
        for ev in raw {
            self.handle_raw(ev, &mut out);
        }
        let actions = self.engine.tick(self.now());
        self.execute(actions);
        out
    }
}

/// Push the resident's preset state to the device: switch the
/// active layer (`switch_layer`, the `[0xA1, layer]` report every
/// assignment write already uses) and, unless this session already
/// wrote it, store the preset's LED effect for that layer.
///
/// The connection is short-lived on purpose: on Linux the config
/// interface is claimed exclusively, so holding it open would block
/// the GUI for as long as the monitor runs. Every failure is logged
/// and leaves the resident's state untouched-but-persisted — the
/// next startup or gesture retries the sync — and an LED failure
/// never undoes a successful layer switch.
fn apply_preset_to_device(preset: u8, led_written: &mut [bool; 4], force_led: bool, reason: &str) {
    let mut device = crate::device::T1Device::new();
    match device.connect() {
        Ok(true) => {}
        Ok(false) => {
            state::log_line(&format!(
                "preset {reason}: device not connected ({}); preset {preset} applies on the next successful sync",
                device.last_error().unwrap_or_default()
            ));
            return;
        }
        Err(err) => {
            state::log_line(&format!(
                "preset {reason}: connect failed: {}; preset {preset} applies on the next successful sync",
                err.message.replace('\n', " ")
            ));
            return;
        }
    }
    if device.switch_layer(preset) {
        state::log_line(&format!("preset -> {preset} ({reason})"));
    } else {
        state::log_line(&format!(
            "preset {reason}: switch to preset {preset} failed: {}",
            device.last_failure().unwrap_or_default()
        ));
    }
    if force_led || !led_written[preset as usize] {
        let mode = preset_gesture::preset_led_mode(preset);
        if device.write_led(preset, mode) {
            led_written[preset as usize] = true;
            state::log_line(&format!(
                "preset {preset} LED effect -> {} (mode {mode})",
                preset_gesture::preset_led_effect(preset)
            ));
        } else {
            state::log_line(&format!(
                "preset {reason}: LED write for preset {preset} failed: {} (the preset switch stands)",
                device.last_failure().unwrap_or_default()
            ));
        }
    }
    device.close();
}

fn dial_step(settings: &Settings) -> f64 {
    settings
        .get("dial")
        .and_then(|v| v.as_object())
        .and_then(|d| d.get("step"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.05)
}

pub fn run_monitor() -> i32 {
    let Some(_lock) = state::acquire_resident_lock() else {
        return 0;
    };
    state::log_line(&format!("monitor starting (pid {})", std::process::id()));
    state::write_pid_file();
    monitor_loop();
    state::remove_pid_file();
    drop(_lock);
    0
}

fn monitor_loop() {
    let mut settings = settings::load_settings();
    let mut mtime = settings::settings_mtime();
    let mut modes = dial_modes(&settings);
    let start = settings
        .get("dial")
        .and_then(|v| v.as_object())
        .and_then(|d| d.get("start_index"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let mut mode_idx = (start as usize) % modes.len().max(1);
    let mut rotate_codes: std::collections::HashSet<u16> =
        tables::DIAL_DEFAULT_CODES.iter().copied().collect();
    let mut push_code: Option<u16> = None;
    let mut source: Option<Box<dyn InputSource>> = None;
    let mut warned_empty = false;
    let mut chord_cfg = pair_chord::PairChordConfig::from_settings(&settings);
    #[cfg(target_os = "linux")]
    let mut chord_pump: Option<ChordPump> = None;
    #[cfg(target_os = "macos")]
    let mut chord_mac_warned = false;
    // Hold-and-twist preset switching. The tracked preset is the
    // persisted `layer` setting, shared with the GUI's preset tabs.
    let mut gesture = PresetGesture::new(preset_gesture::active_preset(&settings));
    // LED effects this monitor session has already stored per preset
    // (index 1..=3); bounds flash writes to one per preset per run.
    let mut led_written = [false; 4];

    let reload = |settings: &mut Settings,
                  modes: &mut Vec<String>,
                  mode_idx: &mut usize,
                  rotate_codes: &mut std::collections::HashSet<u16>,
                  push_code: &mut Option<u16>,
                  chord_cfg: &mut pair_chord::PairChordConfig,
                  gesture: &mut PresetGesture| {
        *settings = settings::load_settings();
        gesture.sync_preset(preset_gesture::active_preset(settings));
        *chord_cfg = pair_chord::PairChordConfig::from_settings(settings);
        #[cfg(windows)]
        input::pair_chord_set_config(chord_cfg.clone());
        *modes = dial_modes(settings);
        if *mode_idx >= modes.len() {
            *mode_idx = 0;
        }
        rotate_codes.clear();
        let dial = settings.get("dial").and_then(|v| v.as_object());
        if let Some(codes) = dial
            .and_then(|d| d.get("rotate_codes"))
            .and_then(|v| v.as_array())
        {
            for code in codes {
                if let Some(code) = code.as_i64() {
                    rotate_codes.insert(code as u16);
                }
            }
        }
        if rotate_codes.is_empty() {
            rotate_codes.extend(tables::DIAL_DEFAULT_CODES.iter().copied());
        }
        *push_code = dial
            .and_then(|d| d.get("push_code"))
            .and_then(|v| v.as_i64())
            .map(|n| n as u16);
    };
    reload(
        &mut settings,
        &mut modes,
        &mut mode_idx,
        &mut rotate_codes,
        &mut push_code,
        &mut chord_cfg,
        &mut gesture,
    );
    // Sync the device with the persisted preset once at startup:
    // active layer plus that layer's LED effect, so the pad and the
    // app agree even if the pad was power-cycled or used elsewhere.
    apply_preset_to_device(gesture.preset(), &mut led_written, true, "startup");

    loop {
        // Pair chords (Linux): while enabled, the grabbed pump replaces
        // the listen-only source. Toggling off shuts the engine down
        // cleanly and releases the grab.
        #[cfg(target_os = "linux")]
        {
            if chord_cfg.enabled && chord_pump.is_none() {
                chord_pump = ChordPump::open(chord_cfg.clone());
                if chord_pump.is_some() {
                    source = None;
                }
            }
            if !chord_cfg.enabled && chord_pump.is_some() {
                if let Some(mut pump) = chord_pump.take() {
                    pump.shutdown();
                }
                source = None;
            }
        }
        #[cfg(target_os = "macos")]
        if chord_cfg.enabled && !chord_mac_warned {
            chord_mac_warned = true;
            state::log_line("pair chords are not supported on macOS yet; ignoring the toggle");
        }
        let pump_active = {
            #[cfg(target_os = "linux")]
            {
                chord_pump.is_some()
            }
            #[cfg(not(target_os = "linux"))]
            {
                false
            }
        };
        if !pump_active && source.is_none() {
            source = input::open_input_source();
            if source.is_none() {
                if !warned_empty {
                    warned_empty = true;
                    state::log_line("no readable input node yet (udev rule / permissions?)");
                }
                thread::sleep(Duration::from_secs(2));
                continue;
            }
            warned_empty = false;
            if let Some(src) = source.as_ref() {
                state::log_line(&format!("monitor attached to {}", src.describe()));
            }
        }
        if settings::settings_mtime() != mtime {
            mtime = settings::settings_mtime();
            reload(
                &mut settings,
                &mut modes,
                &mut mode_idx,
                &mut rotate_codes,
                &mut push_code,
                &mut chord_cfg,
                &mut gesture,
            );
            // A config swap under a running pump: restart it so the new
            // pairs/singles apply (shutdown releases injected keys).
            #[cfg(target_os = "linux")]
            if let Some(mut pump) = chord_pump.take() {
                pump.shutdown();
                if chord_cfg.enabled {
                    chord_pump = ChordPump::open(chord_cfg.clone());
                }
            }
        }
        let events = {
            #[cfg(target_os = "linux")]
            {
                match chord_pump.as_mut() {
                    Some(pump) => pump.pump(0.25),
                    None => match source.as_mut() {
                        Some(src) => src.wait_events(0.25),
                        None => continue,
                    },
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                let Some(src) = source.as_mut() else {
                    continue;
                };
                src.wait_events(0.25)
            }
        };
        for event in events {
            if event.etype == EV_KEY && push_code == Some(event.code) {
                if press_action(&settings) == PressAction::Key {
                    // The press is a written key assignment (K1-C / K2-C);
                    // it must not cycle the volume mode.
                    continue;
                }
                set_dial_held(event.value == 1);
                match event.value {
                    1 => gesture.press_down(),
                    // The volume-mode cycle moved from press-down to
                    // release: a press that twisted is a preset
                    // switch, and only a plain tap still cycles. The
                    // guard's press_up() runs exactly once per release
                    // and resets the hold state either way.
                    0 if gesture.press_up() == preset_gesture::Release::Tap && modes.len() > 1 => {
                        mode_idx = (mode_idx + 1) % modes.len();
                        let dial_obj = settings::ensure_object(&mut settings, "dial");
                        dial_obj.insert("start_index".into(), Value::from(mode_idx as i64));
                        let _ = settings::save_settings(&settings);
                        mtime = settings::settings_mtime();
                        state::log_line(&format!(
                            "dial mode -> {}",
                            modes.get(mode_idx).map(String::as_str).unwrap_or("mic")
                        ));
                    }
                    // Guard failed: the release was a twist (or a
                    // single-mode tap). The guard already consumed
                    // it, so there is nothing left to do.
                    0 => {}
                    _ => {}
                }
                continue;
            }
            if event.etype == EV_REL && rotate_codes.contains(&event.code) && event.value != 0 {
                if knob_is_key_mode(&settings) {
                    // The rotation is a written key assignment
                    // (K1-L / K1-R / K2-L / K2-R); it must not change
                    // the volume.
                    continue;
                }
                if gesture.is_held() {
                    // Hold-and-twist: step the preset instead of the
                    // volume. The rotation is swallowed either way,
                    // so a held twist never leaks volume changes.
                    if let Some(preset) = gesture.rotate(event.value) {
                        settings.insert("layer".into(), Value::from(preset));
                        let _ = settings::save_settings(&settings);
                        mtime = settings::settings_mtime();
                        apply_preset_to_device(preset, &mut led_written, false, "hold-twist");
                    }
                    continue;
                }
                let step = dial_step(&settings);
                let direction = if event.value > 0 { step } else { -step };
                match modes.get(mode_idx).map(String::as_str).unwrap_or("mic") {
                    "headphone" => {
                        volume::set_volume("sink", Some(direction), None);
                    }
                    _ => {
                        volume::set_volume("source", Some(direction), None);
                    }
                }
            }
        }
    }
}

pub fn start_monitor_process() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if !volume::which_wpctl() {
        return Err("wpctl not found (install pipewire-utils)".into());
    }
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let mut command = std::process::Command::new(exe);
    command.arg("--monitor");
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        // The monitor must never show a console window: when the GUI
        // was itself started through the console-subsystem exe,
        // `current_exe` IS that console exe, and without
        // CREATE_NO_WINDOW the child would pop a shell window.
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    }
    command.spawn().map_err(|err| err.to_string())?;
    Ok(())
}

pub fn stop_monitor_process() -> bool {
    if let Some(pid) = state::read_pid() {
        #[cfg(unix)]
        {
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        }
        #[cfg(windows)]
        {
            // taskkill is a console program; spawned from the
            // windowless GUI it would flash a console window without
            // CREATE_NO_WINDOW.
            use std::os::windows::process::CommandExt;
            let mut kill = std::process::Command::new("taskkill");
            kill.args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
            let _ = kill.output();
        }
        state::remove_pid_file();
        return true;
    }
    false
}

pub fn run_watch(seconds: Option<u64>) -> i32 {
    let Some(mut source) = input::open_input_source() else {
        eprintln!("no readable T1 Keyboard input node - check the udev rule / permissions");
        return 1;
    };
    let names = tables::evdev_key_names();
    println!("watching: {}", source.describe());
    println!("press a key or turn the dial (Ctrl-C to stop)");
    let deadline = seconds.map(|s| Instant::now() + Duration::from_secs(s));
    loop {
        if deadline.is_some_and(|end| Instant::now() >= end) {
            break;
        }
        for event in source.wait_events(0.5) {
            if event.etype == tables::EV_SYN {
                continue;
            }
            let label = names
                .get(&(event.etype, event.code))
                .copied()
                .unwrap_or("code");
            let kind = if event.etype == EV_KEY {
                "KEY"
            } else if event.etype == EV_REL {
                "REL"
            } else {
                "EV"
            };
            let state_name = if event.etype == EV_KEY {
                if event.value == 1 {
                    "down"
                } else if event.value == 0 {
                    "up"
                } else {
                    "repeat"
                }
            } else {
                ""
            };
            println!(
                "  {} {:<4} {:<18} {}{}",
                source.describe(),
                kind,
                label,
                state_name,
                if event.etype == EV_KEY {
                    String::new()
                } else {
                    format!("{:+}", event.value)
                }
            );
        }
    }
    0
}

pub fn selftest(write: bool) -> i32 {
    let mut failed = false;
    let mut check = |name: &str, ok: bool, detail: &str| {
        println!(
            "{name:<28} {}{}",
            if ok { "OK  " } else { "FAIL" },
            if detail.is_empty() {
                String::new()
            } else {
                format!(" - {detail}")
            }
        );
        if !ok {
            failed = true;
        }
    };
    check("platform", true, std::env::consts::OS);
    #[cfg(target_os = "linux")]
    {
        let rule = std::path::Path::new("/etc/udev/rules.d/99-t1-keyboard.rules");
        check(
            "udev rule installed",
            rule.exists(),
            if rule.exists() {
                "/etc/udev/rules.d/99-t1-keyboard.rules"
            } else {
                "run: sudo ./install.sh"
            },
        );
        let node = crate::transport::linux::usb_node_path();
        check(
            "usb device present",
            node.is_some(),
            node.as_deref().unwrap_or("1189:8890 not found"),
        );
        if let Some(node) = node.as_deref() {
            let fd =
                unsafe { libc::open(std::ffi::CString::new(node).unwrap().as_ptr(), libc::O_RDWR) };
            let ok = fd >= 0;
            if ok {
                unsafe {
                    libc::close(fd);
                }
            }
            check("usb node permission", ok, node);
        }
    }
    let mut device = crate::device::T1Device::new();
    let connected = match device.connect() {
        Ok(ok) => {
            check(
                "config interface",
                ok,
                device.last_error().unwrap_or_default().as_str(),
            );
            ok
        }
        Err(err) => {
            check("config interface", false, &err.message.replace('\n', " "));
            false
        }
    };
    if connected {
        let frame = if device.had_report_id {
            1 + device.data_len
        } else {
            device.data_len
        };
        check(
            "report id detected",
            true,
            &format!(
                "id={} data={} bytes frame={frame}",
                device.report_id, device.data_len
            ),
        );
        let detail = if device.transport_kind() == "usbfs" {
            let kind = match device.urb_type() {
                Some(1) => "interrupt",
                Some(3) => "bulk",
                _ => "control",
            };
            format!("urb_type={kind}")
        } else {
            format!("hid output report ({})", device.node().unwrap_or_default())
        };
        check("write path", true, &detail);
        if write {
            let ok = device.switch_layer(1) && device.write_payload(&[0xAA, 0xAA]);
            check(
                "test write (preset+flash)",
                ok,
                if ok { "" } else { "failed" },
            );
        }
    }
    #[cfg(target_os = "linux")]
    {
        let nodes = crate::transport::linux::t1_input_nodes();
        let mut readable = 0;
        for (path, _) in &nodes {
            let fd = unsafe {
                libc::open(
                    std::ffi::CString::new(path.as_str()).unwrap().as_ptr(),
                    libc::O_RDONLY | libc::O_NONBLOCK,
                )
            };
            if fd >= 0 {
                readable += 1;
                unsafe {
                    libc::close(fd);
                }
            }
        }
        check(
            "input events readable",
            !nodes.is_empty() && readable == nodes.len(),
            &format!("{readable}/{}", nodes.len()),
        );
    }
    device.close();
    println!("\nRESULT: {}", if failed { "FAIL" } else { "PASS" });
    if failed { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings_with(value: Option<&str>) -> Settings {
        let mut settings = Settings::new();
        if let Some(value) = value {
            settings.insert("dial".into(), json!({ "press_action": value }));
        }
        settings
    }

    #[test]
    fn press_action_defaults_to_cycle() {
        assert_eq!(press_action(&settings_with(None)), PressAction::Cycle);
        assert_eq!(
            press_action(&settings_with(Some("cycle"))),
            PressAction::Cycle
        );
        assert_eq!(
            press_action(&settings_with(Some("bogus"))),
            PressAction::Cycle
        );
    }

    #[test]
    fn press_action_key_only_for_key() {
        assert_eq!(press_action(&settings_with(Some("key"))), PressAction::Key);
    }

    #[test]
    fn knob_key_mode_covers_press_and_rotation() {
        // The rotation guard in the monitor loop uses the same decision
        // as the press guard: key mode means the resident ignores both.
        assert!(!knob_is_key_mode(&settings_with(None)));
        assert!(!knob_is_key_mode(&settings_with(Some("cycle"))));
        assert!(!knob_is_key_mode(&settings_with(Some("bogus"))));
        assert!(knob_is_key_mode(&settings_with(Some("key"))));
    }
}
