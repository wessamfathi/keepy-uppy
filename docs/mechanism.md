# How keepy-uppy keeps a closed MacBook awake (macOS)

Verified on macOS 27.0.1, Apple Silicon, battery and AC, no external display (2026-10-27).

## The call
```
IOServiceOpen(IOServiceGetMatchingService(0, IOServiceMatching("IOPMrootDomain")), mach_task_self(), 0, &conn)
IOConnectCallScalarMethod(conn, 12 /* kPMSetClamshellSleepState */, &(uint64_t){1 or 0}, 1, NULL, NULL)
```
- No root, entitlement or helper is needed. xnu `RootDomainUserClient.cpp` dispatches selector 12 without
  a privilege check and sets `kClamshellSleepDisablePowerd` in `clamshellSleepDisableMask`.
- While the mask is non-zero, `IOPMrootDomain::shouldSleepOnClamshellClosed()` returns false.
  Closing the lid turns the internal display off but doesn't sleep the Mac.
- This is a private, undocumented interface and can break in any macOS release. It's incompatible with the Mac App Store sandbox.

## powerd shares the bit
powerd (`pmconfigd/PMAssertions.c: setClamshellSleepState`) writes the same bit whenever its own computed
state flips between 0 and non-zero. Its state is non-zero while some process holds an assertion flagged as
applying on lid close (Apple-entitlement only), and after every sleep. When that state drops back to 0, powerd writes 0
and our arm is gone. If the lid is closed at that moment, the kernel re-evaluates synchronously and the Mac
sleeps ~14 ms later, faster than any user-space process can react.

Consequences for the app:
1. **Re-arm periodically** (1 s) and on power-source change and wake. This covers clears that happen while the lid is open.
2. **The charger chime is the lid-closed trigger.** PowerChime holds a `DisplayWake "com.apple.chimeOnAttach"`
   assertion for ~1 s when the charger is attached. Its release makes powerd write 0. With
   `defaults write com.apple.PowerChime ChimeOnNoHardware -bool true`, PowerChime still launches on the
   power event but exits without an assertion, so the problem goes away. Offer this as an opt-in toggle and restore the setting when it's turned off.
3. **The bit outlives the process.** The kernel doesn't clear it when we exit. Disarm on quit and signals, and
   always disarm at launch to recover from a crash.

## Test matrix (spike `hold` mode, re-arm every 1 s)
| Scenario | Result |
|---|---|
| Lid closed, battery | awake |
| Lid closed, disarmed | sleeps after ~5 s (correct) |
| Lid open → plug charger → close lid | awake |
| Lid closed → unplug charger | awake (powerd makes no write) |
| Lid closed → plug charger, chime on | **sleeps** |
| Lid closed → plug charger, chime off | awake |
| Arm, process exits, close lid | awake (bit persists) |
| Reboot while armed | TODO |
| External display hot-plug while lid closed | TODO (WindowServer hot-plug assertion likely has the same effect) |

## Verifying
- Trust `pmset -g log` ("Entering Sleep state due to 'Clamshell Sleep'") and
  `/usr/bin/log show --info --predicate 'process == "powerd"'` (EvaluateClamshell lines).
- Don't trust `ioreg ... AppleClamshellCausesSleep`. It is cached and reads `No` even after a real clamshell sleep.

## Rejected
- `caffeinate` / IOPMAssertion: stops idle sleep only.
- `AppliesOnLidClose` assertion property: needs an Apple-private entitlement.
- `pmset -a disablesleep 1`: robust (powerd doesn't touch it) but needs root and persists across reboot.
  Possible future opt-in "robust mode" via an SMAppService helper.
