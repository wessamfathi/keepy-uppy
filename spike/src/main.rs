//! Throwaway spike: toggle IOPMrootDomain clamshell-sleep-disable without root.
//!
//! usage: ku-spike arm | disarm | hold
//!   hold = arm, wait for Ctrl-C, then disarm.

use std::ffi::{c_char, c_void};
use std::process::ExitCode;

type KernReturn = i32;
type MachPort = u32;
type IoObject = MachPort;
type IoConnect = MachPort;

const KERN_SUCCESS: KernReturn = 0;
const K_PM_SET_CLAMSHELL_SLEEP_STATE: u32 = 12;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main_port: MachPort, matching: *mut c_void) -> IoObject;
    fn IOServiceOpen(service: IoObject, owning_task: MachPort, kind: u32, connect: *mut IoConnect) -> KernReturn;
    fn IOServiceClose(connect: IoConnect) -> KernReturn;
    fn IOObjectRelease(object: IoObject) -> KernReturn;
    fn IOConnectCallScalarMethod(
        connection: IoConnect,
        selector: u32,
        input: *const u64,
        input_cnt: u32,
        output: *mut u64,
        output_cnt: *mut u32,
    ) -> KernReturn;
}

extern "C" {
    static mach_task_self_: MachPort;
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    fn pause() -> i32;
}

fn set_clamshell_sleep_disabled(disabled: bool) -> Result<(), String> {
    unsafe {
        let matching = IOServiceMatching(c"IOPMrootDomain".as_ptr());
        let service = IOServiceGetMatchingService(0, matching);
        if service == 0 {
            return Err("IOPMrootDomain not found".into());
        }
        let mut conn: IoConnect = 0;
        let kr = IOServiceOpen(service, mach_task_self_, 0, &mut conn);
        IOObjectRelease(service);
        if kr != KERN_SUCCESS {
            return Err(format!("IOServiceOpen failed: {kr:#x}"));
        }
        let input = [disabled as u64];
        let kr = IOConnectCallScalarMethod(
            conn,
            K_PM_SET_CLAMSHELL_SLEEP_STATE,
            input.as_ptr(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        IOServiceClose(conn);
        if kr != KERN_SUCCESS {
            return Err(format!("IOConnectCallScalarMethod failed: {kr:#x}"));
        }
    }
    Ok(())
}

extern "C" fn on_signal(_: i32) {}

fn main() -> ExitCode {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let result = match cmd.as_str() {
        "arm" => set_clamshell_sleep_disabled(true),
        "disarm" => set_clamshell_sleep_disabled(false),
        "hold" => set_clamshell_sleep_disabled(true).and_then(|()| {
            println!("armed; Ctrl-C to disarm");
            unsafe {
                signal(2, on_signal);
                signal(15, on_signal);
                pause();
            }
            set_clamshell_sleep_disabled(false)
        }),
        _ => {
            eprintln!("usage: ku-spike arm | disarm | hold");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => {
            println!("{cmd}: ok");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{cmd}: {e}");
            ExitCode::FAILURE
        }
    }
}
