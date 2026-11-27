//! Command line. The default is the GUI. `--monitor` stays resident and
//! turns dial rotation into OS volume changes.

use clap::Parser;

use crate::resident;
use crate::{APP_NAME, VERSION};

#[derive(Parser, Debug)]
#[command(name = APP_NAME, version = VERSION, about = crate::edition::CLI_ABOUT)]
struct Args {
    /// Run the dial volume listener.
    #[arg(long)]
    monitor: bool,
    /// Check device access and the write path.
    #[arg(long)]
    selftest: bool,
    /// With --selftest, perform a harmless test write.
    #[arg(long)]
    write: bool,
    /// Print T1 input events. Omit the seconds to run until Ctrl-C.
    #[arg(long, num_args = 0..=1, default_missing_value = "0")]
    watch: Option<u64>,
    /// Start the GUI (this is the default).
    #[arg(long)]
    gui: bool,
}

pub fn run(argv: &[String]) -> i32 {
    if std::env::var_os("T1_FONT_CHECK").is_some() {
        let proof = std::env::var_os("T1_FONT_PROOF").map(std::path::PathBuf::from);
        return crate::gui::font_check(proof.as_deref());
    }
    let args = match Args::try_parse_from(
        std::iter::once(APP_NAME.to_string()).chain(argv.iter().cloned()),
    ) {
        Ok(args) => args,
        Err(err) => {
            let _ = err.print();
            return if err.use_stderr() { 2 } else { 0 };
        }
    };
    if args.monitor {
        return resident::run_monitor();
    }
    if let Some(seconds) = args.watch {
        return resident::run_watch(if seconds == 0 { None } else { Some(seconds) });
    }
    if args.selftest {
        return resident::selftest(args.write);
    }
    if let Some(code) = gui_handoff() {
        return code;
    }
    crate::gui::run_gui()
}

/// Whether a process that is about to open the GUI should hand off to
/// the GUI exe instead: true when this process is NOT already the
/// GUI exe and a sibling GUI exe exists. The console-subsystem exe
/// (`t1-keyboard-config.exe`) also opens the GUI with no arguments,
/// but it would keep its console window on screen for the whole
/// session — and the monitor child it spawns via `current_exe` would
/// inherit the problem.
pub fn should_handoff_to_gui(exe_name: &str, sibling_gui_exists: bool) -> bool {
    sibling_gui_exists && !exe_name.eq_ignore_ascii_case("t1-keyboard-config-gui.exe")
}

/// Windows: hand the GUI to the sibling `t1-keyboard-config-gui.exe`
/// (windows subsystem, no console) and exit. Returns the exit code
/// when the handoff happened, or None to open the GUI in-process
/// (already the GUI exe, no sibling, or the spawn failed).
#[cfg(windows)]
fn gui_handoff() -> Option<i32> {
    let exe = std::env::current_exe().ok()?;
    let name = exe.file_name()?.to_str()?;
    let sibling = exe.with_file_name("t1-keyboard-config-gui.exe");
    if !should_handoff_to_gui(name, sibling.is_file()) {
        return None;
    }
    use std::os::windows::process::CommandExt;
    let spawned = std::process::Command::new(&sibling)
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0)
        .spawn();
    match spawned {
        Ok(_) => Some(0),
        Err(_) => None,
    }
}

#[cfg(not(windows))]
fn gui_handoff() -> Option<i32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_only_from_the_console_exe_with_a_gui_sibling() {
        assert!(should_handoff_to_gui("t1-keyboard-config.exe", true));
        assert!(!should_handoff_to_gui("t1-keyboard-config.exe", false));
        assert!(!should_handoff_to_gui("t1-keyboard-config-gui.exe", true));
        assert!(!should_handoff_to_gui("T1-Keyboard-Config-GUI.exe", true));
    }
}
