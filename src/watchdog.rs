//! Crash fail-safe. The kernel keeps our bit after the app dies (crash,
//! `kill -9`, Force Quit), which would leave a closed Mac awake in a bag.
//!
//! The app runs a copy of itself as `keepy-uppy --watchdog` and sends its
//! armed state (`1`/`0`) down a pipe every tick. When the app dies for any
//! reason the kernel closes the pipe; the watchdog sees EOF and disarms if the
//! last state it heard was armed.

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use crate::platform::{Controller, SleepController};

pub const ARG: &str = "--watchdog";

extern "C" {
    fn setsid() -> i32;
}

/// Entry point of the watchdog process.
pub fn run() -> ! {
    // Own session: Ctrl-C in the app's terminal must not take us down with it.
    unsafe { setsid() };

    let mut armed = false;
    let mut stdin = std::io::stdin().lock();
    let mut byte = [0u8; 1];
    // EOF (0) or an error means the app is gone.
    while let Ok(1) = stdin.read(&mut byte) {
        armed = byte[0] == b'1';
    }
    if armed {
        let result = Controller::new().and_then(|mut c| c.disarm());
        match result {
            Ok(()) => eprintln!("keepy-uppy watchdog: app exited while armed; disarmed"),
            Err(e) => eprintln!("keepy-uppy watchdog: {e}"),
        }
    }
    std::process::exit(0);
}

/// The app's handle on its watchdog.
pub struct Watchdog {
    child: Option<(Child, ChildStdin)>,
}

impl Watchdog {
    pub fn start() -> Self {
        let mut w = Self { child: None };
        w.spawn();
        w
    }

    fn spawn(&mut self) {
        let spawned = std::env::current_exe().and_then(|exe| {
            Command::new(exe)
                .arg(ARG)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
        });
        match spawned {
            Ok(mut child) => {
                let stdin = child.stdin.take().expect("stdin is piped");
                self.child = Some((child, stdin));
            }
            Err(e) => eprintln!("keepy-uppy: watchdog: {e}"),
        }
    }

    /// Call every tick and on every change. Restarts the watchdog if it died.
    pub fn report(&mut self, armed: bool) {
        let byte = if armed { b"1" } else { b"0" };
        if let Some((_, stdin)) = &mut self.child {
            if stdin.write_all(byte).is_ok() {
                return;
            }
        }
        if let Some((mut child, _)) = self.child.take() {
            let _ = child.try_wait(); // reap it
        }
        self.spawn();
        if let Some((_, stdin)) = &mut self.child {
            let _ = stdin.write_all(byte);
        }
    }
}
