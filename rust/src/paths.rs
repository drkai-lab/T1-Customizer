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
    state_dir().join("monitor.log")
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
