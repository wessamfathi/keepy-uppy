# Keepy Uppy

Keep your MacBook running with the lid closed. No external display, no admin password.

Music keeps playing, builds keep compiling, AI agents keep working. The internal display turns off as usual; only the sleep-on-lid-close is skipped.

Free and open source (GPL-3.0). macOS 14 or later, Apple Silicon.

## Install

Download the latest `keepy-uppy-<version>.dmg` from [Releases](https://github.com/wessamfathi/keepy-uppy/releases), open it and drag **Keepy Uppy** to Applications. The app is signed with a Developer ID and notarized by Apple.

Keepy Uppy lives in the menu bar (no Dock icon). The icon is a laptop lid with an eye: **open eye** = awake with the lid closed, **closed eye** = normal sleep.

## Using it

Click the menu bar icon:

| Item | What it does |
|---|---|
| Keep awake until disarmed | Lid close won't sleep the Mac until you choose Disarm |
| Keep awake for 1 hour / 4 hours | Same, then disarms automatically |
| Disarm | Lid close sleeps the Mac again |
| Disarm on low battery | On battery, disarm at 10% (default), 20%, 30% or never |
| Mute charger chime | Fixes the Mac sleeping when you plug in the charger with the lid closed (see below) |
| Launch at login | Start Keepy Uppy when you log in (it always starts disarmed) |
| Check for updates | Looks up the latest GitHub release; only goes online when you click it |

### The charger chime

With the lid closed, plugging in the charger makes macOS play its "connected" chime. The chime briefly holds a power assertion, and when it ends macOS re-evaluates lid-close sleep and puts the Mac to sleep before any app can react. **Mute charger chime** turns that chime off (`com.apple.PowerChime ChimeOnNoHardware`), which stops it. Untick it to restore the macOS default. Unplugging the charger is not affected.

## Safety

A Mac that can't sleep with its lid closed can overheat in a bag, so Keepy Uppy is careful about turning itself off:

- **Always starts disarmed.** Arming is always your choice; the armed state is never restored at launch.
- **Disarms when it stops:** on Quit, Ctrl-C and `kill`.
- **Disarms if it crashes.** A small watchdog process notices when the app dies (crash, Force Quit, `kill -9`) and turns lid-close sleep back on.
- **Disarms on low battery** (on battery power only) and when macOS reports **serious or critical heat**.
- **Timers end on time**, and any automatic disarm ends the session; you re-arm yourself.

Settings live in `~/Library/Application Support/keepy-uppy/config.toml`.

## How it works

macOS sleeps on lid close unless a kernel flag says otherwise. Keepy Uppy sets that flag through the same private power-management call macOS's own power daemon uses (`IOPMrootDomain`, selector `kPMSetClamshellSleepState`). That call needs no root and no entitlement, so there is no helper, no `sudo`, and no `pmset disablesleep` left behind.

The flag is shared with `powerd`, which can clear it, so the app re-asserts it every second while armed. The kernel keeps the flag after the app exits, which is why the app disarms at launch and runs the crash watchdog.

Full details, on-device test results and known limits: [docs/mechanism.md](docs/mechanism.md).

### Limits

- Uses a private macOS interface. A future macOS release could change or remove it.
- Plugging in an external display with the lid closed is untested.
- Apple Silicon only for now.

## Building from source

Needs Rust (stable) and Xcode command line tools.

```sh
cargo run                      # run from the terminal
cargo test                     # policy, config, update and icon tests
sh scripts/bundle.sh           # build "target/release/Keepy Uppy.app" (ad-hoc signed)
```

Release DMG (Developer ID signed, notarized, stapled):

```sh
DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)" \
NOTARY_PROFILE=keepy-uppy \
sh scripts/release.sh
```

See the comments in `scripts/release.sh` for the one-time `notarytool` setup.

Icons are drawn in code: `scripts/draw-menubar-icons.swift` (menu bar) and `scripts/build-app-icon.sh` (app icon).

### Layout

| Path | Contents |
|---|---|
| `src/platform/macos.rs` | IOKit call, battery and power source, thermal state, charger chime, login item |
| `src/policy.rs` | Pure arm/disarm rules (unit-tested) |
| `src/watchdog.rs` | Crash fail-safe |
| `src/tray.rs` | Menu bar menu |
| `src/main.rs` | Event loop and 1 s tick |
| `spike/` | The original proof-of-concept CLI (`arm`, `disarm`, `hold`) |

## License

[GPL-3.0](LICENSE). Forks and derivatives must stay open source.
