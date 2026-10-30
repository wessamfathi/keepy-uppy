//! OS-specific code. Everything outside this module is platform-neutral.

#[cfg(not(target_os = "macos"))]
compile_error!("keepy-uppy only supports macOS for now");

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::{
    chime_muted, config_dir, install_stop_signals, launch_at_login, power, set_chime_muted,
    set_launch_at_login, stop_requested, thermal, ClamshellController as Controller,
};

pub trait SleepController {
    /// Stop lid close from sleeping the machine. Safe to call repeatedly.
    fn arm(&mut self) -> Result<(), String>;
    /// Restore normal lid-close sleep.
    fn disarm(&mut self) -> Result<(), String>;
    /// Whether our last successful call was `arm`.
    fn is_armed(&self) -> bool;
}
