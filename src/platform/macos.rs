//! macOS backend. See docs/mechanism.md for why each piece exists.

use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use objc2_foundation::{NSProcessInfo, NSProcessInfoThermalState};
use objc2_service_management::{SMAppService, SMAppServiceStatus};

use super::SleepController;
use crate::policy::{Power, Thermal};

type KernReturn = i32;
type MachPort = u32;
type IoObject = MachPort;
type IoConnect = MachPort;

const KERN_SUCCESS: KernReturn = 0;
/// RootDomainUserClient selector; sets the kernel's powerd clamshell-sleep-disable bit.
const K_PM_SET_CLAMSHELL_SLEEP_STATE: u32 = 12;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main_port: MachPort, matching: *mut c_void) -> IoObject;
    fn IOServiceOpen(
        service: IoObject,
        owning_task: MachPort,
        kind: u32,
        connect: *mut IoConnect,
    ) -> KernReturn;
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

    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, ps: CFTypeRef) -> CFDictionaryRef;
    fn IOPSGetProvidingPowerSourceType(snapshot: CFTypeRef) -> CFStringRef;
}

extern "C" {
    static mach_task_self_: MachPort;
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
}

const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_stop_signal(_: i32) {
    // Only an atomic store here; the event loop disarms on its next tick.
    STOP.store(true, Ordering::SeqCst);
}

/// SIGINT/SIGTERM (Ctrl-C, `kill`, logout) request a clean disarm and exit.
pub fn install_stop_signals() {
    unsafe {
        signal(SIGINT, on_stop_signal);
        signal(SIGTERM, on_stop_signal);
    }
}

pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// Holds an `IOPMrootDomain` user-client connection for the app's lifetime.
pub struct ClamshellController {
    conn: IoConnect,
    armed: bool,
}

impl ClamshellController {
    pub fn new() -> Result<Self, String> {
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
            Ok(Self { conn, armed: false })
        }
    }

    fn set(&mut self, disabled: bool) -> Result<(), String> {
        let input = [disabled as u64];
        let kr = unsafe {
            IOConnectCallScalarMethod(
                self.conn,
                K_PM_SET_CLAMSHELL_SLEEP_STATE,
                input.as_ptr(),
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if kr != KERN_SUCCESS {
            return Err(format!("IOConnectCallScalarMethod failed: {kr:#x}"));
        }
        self.armed = disabled;
        Ok(())
    }
}

impl SleepController for ClamshellController {
    fn arm(&mut self) -> Result<(), String> {
        self.set(true)
    }

    fn disarm(&mut self) -> Result<(), String> {
        self.set(false)
    }

    fn is_armed(&self) -> bool {
        self.armed
    }
}

impl Drop for ClamshellController {
    fn drop(&mut self) {
        unsafe { IOServiceClose(self.conn) };
    }
}

/// Current power source and internal battery percentage, via IOPowerSources.
pub fn power() -> Power {
    unsafe {
        let blob = IOPSCopyPowerSourcesInfo();
        if blob.is_null() {
            return Power {
                on_ac: true,
                battery_pct: None,
            };
        }
        let blob = CFType::wrap_under_create_rule(blob);
        let source = IOPSGetProvidingPowerSourceType(blob.as_CFTypeRef());
        let on_ac = source.is_null() || CFString::wrap_under_get_rule(source) == "AC Power";

        let mut battery_pct = None;
        let list = IOPSCopyPowerSourcesList(blob.as_CFTypeRef());
        if !list.is_null() {
            let list: CFArray<CFType> = CFArray::wrap_under_create_rule(list);
            for ps in list.iter() {
                let desc = IOPSGetPowerSourceDescription(blob.as_CFTypeRef(), ps.as_CFTypeRef());
                if desc.is_null() {
                    continue;
                }
                let desc: CFDictionary<CFString, CFType> = CFDictionary::wrap_under_get_rule(desc);
                let num = |key: &'static str| {
                    desc.find(CFString::from_static_string(key))
                        .and_then(|v| v.downcast::<CFNumber>())
                        .and_then(|n| n.to_i64())
                };
                if let (Some(cur), Some(max)) = (num("Current Capacity"), num("Max Capacity")) {
                    if max > 0 {
                        battery_pct = Some((cur * 100 / max).clamp(0, 100) as u8);
                        break;
                    }
                }
            }
        }
        Power { on_ac, battery_pct }
    }
}

pub fn thermal() -> Thermal {
    match NSProcessInfo::processInfo().thermalState() {
        NSProcessInfoThermalState::Nominal => Thermal::Nominal,
        NSProcessInfoThermalState::Fair => Thermal::Fair,
        NSProcessInfoThermalState::Serious => Thermal::Serious,
        _ => Thermal::Critical,
    }
}

/// Registered as a login item (possibly still awaiting approval in System Settings).
pub fn launch_at_login() -> bool {
    let status = unsafe { SMAppService::mainAppService().status() };
    status == SMAppServiceStatus::Enabled || status == SMAppServiceStatus::RequiresApproval
}

/// Only works from a bundled `.app` (scripts/bundle.sh), not `cargo run`.
pub fn set_launch_at_login(on: bool) -> Result<(), String> {
    let service = unsafe { SMAppService::mainAppService() };
    let result = unsafe {
        if on {
            service.registerAndReturnError()
        } else {
            service.unregisterAndReturnError()
        }
    };
    result.map_err(|e| format!("login item: {}", e.localizedDescription()))?;
    if unsafe { service.status() } == SMAppServiceStatus::RequiresApproval {
        unsafe { SMAppService::openSystemSettingsLoginItems() };
    }
    Ok(())
}

/// `~/Library/Application Support/keepy-uppy`.
pub fn config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/keepy-uppy"))
}

const CHIME_DOMAIN: &str = "com.apple.PowerChime";
const CHIME_KEY: &str = "ChimeOnNoHardware";

/// Whether the charger chime is muted. PowerChime's chime assertion makes
/// powerd clear our bit when the charger is plugged in with the lid closed.
pub fn chime_muted() -> bool {
    Command::new("/usr/bin/defaults")
        .args(["read", CHIME_DOMAIN, CHIME_KEY])
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "1")
        .unwrap_or(false)
}

/// Unmuting deletes the key, restoring the macOS default.
pub fn set_chime_muted(muted: bool) -> Result<(), String> {
    let args: &[&str] = if muted {
        &["write", CHIME_DOMAIN, CHIME_KEY, "-bool", "true"]
    } else {
        &["delete", CHIME_DOMAIN, CHIME_KEY]
    };
    let out = Command::new("/usr/bin/defaults")
        .args(args)
        .output()
        .map_err(|e| format!("defaults: {e}"))?;
    // `delete` fails when the key is already absent, which is the goal anyway.
    if muted && !out.status.success() {
        return Err(format!(
            "defaults write failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    // PowerChime re-reads its preferences when launchd next starts it.
    let _ = Command::new("/usr/bin/killall").arg("PowerChime").output();
    if chime_muted() != muted {
        return Err("chime setting did not change".into());
    }
    Ok(())
}
