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
    crate::gui::run_gui()
}
