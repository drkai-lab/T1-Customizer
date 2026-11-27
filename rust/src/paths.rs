//! Config and state directories. They match the Python app so existing
//! settings load from the same place.

use std::path::{Path, PathBuf};

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("T1_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    if cfg!(windows) {
        let root = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home);
        root.join("t1-keyboard")
    } else if cfg!(target_os = "macos") {
        home().join("Library/Application Support/t1-keyboard")
    } else {
        home().join(".config/t1-keyboard")
    }
}

pub fn state_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("T1_STATE_DIR") {
        return PathBuf::from(dir);
    }
    if cfg!(windows) || cfg!(target_os = "macos") {
        config_dir().join("state")
    } else {
        home().join(".local/state/t1-keyboard")
    }
}

pub fn config_file() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn log_file() -> PathBuf {
    // An explicit state-dir override (tests, debugging) wins, matching
    // state_dir()'s own rule.
    if std::env::var_os("T1_STATE_DIR").is_some() {
        return state_dir().join("monitor.log");
    }
    // Otherwise the log sits next to the executable, where the user can
    // find it without knowing about the state directory. An installed
    // location (e.g. Program Files) may not be writable, so probe once
    // and fall back to the state directory: logging must never silently
    // die just because the exe folder is read-only.
    static CHOSEN: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CHOSEN
        .get_or_init(|| {
            let fallback = state_dir();
            match std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
            {
                Some(dir) => select_log_file(&dir, dir_is_writable(&dir), &fallback),
                None => fallback.join("monitor.log"),
            }
        })
        .clone()
}

/// Which `monitor.log` to use: the executable's own folder when it is
/// writable, the state directory otherwise. Pure, so the choice itself
/// is unit-testable without touching the real exe location.
pub fn select_log_file(exe_dir: &Path, exe_dir_writable: bool, fallback_dir: &Path) -> PathBuf {
    if exe_dir_writable {
        exe_dir.join("monitor.log")
    } else {
        fallback_dir.join("monitor.log")
    }
}

fn dir_is_writable(dir: &Path) -> bool {
    // Probe with a throwaway file; never touch an existing monitor.log.
    let probe = dir.join(format!(".t1-log-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

pub fn pid_file() -> PathBuf {
    state_dir().join("monitor.pid")
}

pub fn lock_file() -> PathBuf {
    state_dir().join("monitor.lock")
}

pub fn state_file(name: &str) -> PathBuf {
    state_dir().join(name)
}

pub fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_file_prefers_the_exe_folder_when_writable() {
        let chosen = select_log_file(
            Path::new("/apps/t1"),
            true,
            Path::new("/home/user/.local/state/t1-keyboard"),
        );
        assert_eq!(chosen, Path::new("/apps/t1/monitor.log"));
    }

    #[test]
    fn log_file_falls_back_to_the_state_dir_when_exe_folder_is_read_only() {
        let chosen = select_log_file(
            Path::new("C:/Program Files/T1-Customizer"),
            false,
            Path::new("C:/Users/user/AppData/Roaming/t1-keyboard/state"),
        );
        assert_eq!(
            chosen,
            Path::new("C:/Users/user/AppData/Roaming/t1-keyboard/state/monitor.log")
        );
    }
}
