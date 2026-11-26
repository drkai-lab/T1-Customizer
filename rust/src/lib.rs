//! T1-Customizer — free configuration tool for the T1 mini keyboard.
//!
//! Forked from T1-VREmoKey by AppWizz Kai (version 2.2.4). The original's
//! network/expression features and its paid gate are removed; the T1
//! hardware configuration core is unchanged, so existing `settings.json`
//! files for the remaining fields still load.

pub mod app_presets;
pub mod cli;
pub mod device;
pub mod edition;
pub mod gui;
pub mod hidparse;
pub mod input;
pub mod keys;
pub mod paths;
pub mod protocol;
pub mod resident;
pub mod settings;
pub mod state;
pub mod tables;
pub mod transport;
pub mod volume;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = "t1-keyboard-config";
pub const VID: u16 = 0x1189;
pub const PID: u16 = 0x8890;
pub const CONFIG_INTERFACE: u8 = 1;
pub const CONFIG_ENDPOINT: u8 = 0x02;

pub use settings::Settings;

#[cfg(test)]
mod version_tests {
    #[test]
    fn version_matches_cargo_toml() {
        assert_eq!(super::VERSION, env!("CARGO_PKG_VERSION"));
    }
}
