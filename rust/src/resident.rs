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
    /// The press is a plain key: K1-C / K2-C carry a written key assignment
    /// and the monitor ignores the press entirely.
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

    let reload = |settings: &mut Settings,
                  modes: &mut Vec<String>,
                  mode_idx: &mut usize,
                  rotate_codes: &mut std::collections::HashSet<u16>,
                  push_code: &mut Option<u16>| {
        *settings = settings::load_settings();
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
    );

    loop {
        if source.is_none() {
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
            );
        }
        let Some(src) = source.as_mut() else {
            continue;
        };
        let events = src.wait_events(0.25);
        for event in events {
            if event.etype == EV_KEY && push_code == Some(event.code) {
                if press_action(&settings) == PressAction::Key {
                    // The press is a written key assignment (K1-C / K2-C);
                    // it must not cycle the volume mode.
                    continue;
                }
                set_dial_held(event.value == 1);
                if event.value == 1 && modes.len() > 1 {
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
                continue;
            }
            if event.etype == EV_REL && rotate_codes.contains(&event.code) && event.value != 0 {
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
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output();
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
}
