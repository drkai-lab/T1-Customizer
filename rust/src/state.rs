//! Resident-process bookkeeping: log, pid file, lock, and status JSON.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::paths;

pub fn log_line(message: &str) {
    let Ok(()) = fs::create_dir_all(paths::state_dir()) else {
        return;
    };
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths::log_file())
    else {
        return;
    };
    let stamp = local_stamp();
    let _ = writeln!(file, "{stamp} {message}");
}

fn local_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Local time without an extra crate. UTC is acceptable for the log if
    // tz info is missing; we still format YYYY-MM-DD HH:MM:SS.
    let offset = local_offset_secs();
    format_epoch(secs + offset)
}

fn local_offset_secs() -> i64 {
    #[cfg(unix)]
    {
        unsafe {
            let mut tm: libc::tm = std::mem::zeroed();
            tm.tm_isdst = -1;
            let now = libc::time(std::ptr::null_mut());
            if libc::localtime_r(&now, &mut tm).is_null() {
                0
            } else {
                tm.tm_gmtoff
            }
        }
    }
    #[cfg(not(unix))]
    {
        0
    }
}

fn format_epoch(secs: i64) -> String {
    let mut secs = secs;
    let days = secs.div_euclid(86_400);
    secs = secs.rem_euclid(86_400);
    let hour = secs / 3600;
    let min = (secs % 3600) / 60;
    let sec = secs % 60;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}")
}

pub fn write_json_atomic(path: &Path, value: &Value) -> bool {
    let Ok(()) = paths::ensure_parent(path) else {
        return false;
    };
    let Ok(text) = serde_json::to_vec(value) else {
        return false;
    };
    let tmp = path.with_extension("json.tmp");
    let Ok(mut file) = File::create(&tmp) else {
        return false;
    };
    if file.write_all(&text).is_err() {
        let _ = fs::remove_file(&tmp);
        return false;
    }
    fs::rename(&tmp, path).is_ok()
}

pub fn read_json(path: &Path) -> Option<Map<String, Value>> {
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value.as_object().cloned()
}

pub fn write_state(name: &str, value: &Value) -> bool {
    write_json_atomic(&paths::state_file(name), value)
}

pub fn read_state(name: &str) -> Option<Map<String, Value>> {
    read_json(&paths::state_file(name))
}

pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        unsafe {
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                .map(|h| {
                    let _ = h;
                    true
                })
                .unwrap_or(false)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

pub fn read_pid() -> Option<u32> {
    let text = fs::read_to_string(paths::pid_file()).ok()?;
    let pid = text.trim().parse::<u32>().ok()?;
    if pid > 0 && pid_alive(pid) {
        Some(pid)
    } else {
        None
    }
}

pub fn write_pid_file() {
    let _ = paths::ensure_parent(&paths::pid_file());
    let _ = fs::write(paths::pid_file(), std::process::id().to_string());
}

pub fn remove_pid_file() {
    let _ = fs::remove_file(paths::pid_file());
}

pub struct ResidentLock {
    _file: File,
}

pub fn acquire_resident_lock() -> Option<ResidentLock> {
    if let Some(pid) = read_pid()
        && pid != std::process::id()
    {
        log_line(&format!("already running (pid {pid})"));
        return None;
    }
    if fs::create_dir_all(paths::state_dir()).is_err() {
        return Some(ResidentLock {
            _file: File::open("/dev/null").unwrap_or_else(|_| tempfile_fallback()),
        });
    }
    let Ok(file) = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(paths::lock_file())
    else {
        return Some(ResidentLock {
            _file: tempfile_fallback(),
        });
    };
    #[cfg(unix)]
    {
        use fs2::FileExt;
        if file.try_lock_exclusive().is_err() {
            log_line("already running (lock held)");
            return None;
        }
    }
    Some(ResidentLock { _file: file })
}

fn tempfile_fallback() -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(paths::lock_file())
        .or_else(|_| File::create(paths::lock_file()))
        .unwrap_or_else(|_| File::open(".").expect("cwd"))
}

pub fn monitor_is_running() -> bool {
    if read_pid().is_some() {
        return true;
    }
    if paths::pid_file().exists() {
        remove_pid_file();
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = std::process::Command::new("pgrep")
            .args(["-f", "t1-keyboard-config --monitor"])
            .output()
        {
            return !out.stdout.is_empty()
                && String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .any(|line| line.trim().parse::<u32>().ok() != Some(std::process::id()));
        }
    }
    false
}
