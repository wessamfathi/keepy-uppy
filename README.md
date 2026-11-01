# Keepy Uppy

Keepy Uppy keeps a MacBook running with the lid closed. You don't need an external display or an admin password.

Spotify keeps playing and long builds or AI agents keep going while the laptop sits closed. The internal display still turns off as usual. The only thing Keepy Uppy changes is the sleep that normally follows closing the lid.

It's free and open source under GPL-3.0, and runs on Apple Silicon Macs with macOS 14 or later.

## Install

Download the latest `keepy-uppy-<version>.dmg` from [Releases](https://github.com/wessamfathi/keepy-uppy/releases), open it and drag Keepy Uppy to Applications. The app is signed with a Developer ID and notarized by Apple.

Keepy Uppy lives in the menu bar and has no Dock icon. The menu bar icon is a laptop lid with an eye on it. An open eye means the Mac will stay awake when you close the lid; a closed eye means it will sleep normally.

## Using it

Click the menu bar icon:

| Item | What it does |
|---|---|
| Keep awake until disarmed | Closing the lid won't sleep the Mac until you choose Disarm |
| Keep awake for 1 hour / 4 hours | Same, then disarms automatically |
| Disarm | Closing the lid sleeps the Mac again |
| Disarm on low battery | On battery, disarms at 10% (default), 20%, 30% or never |
| Mute charger chime | Stops the Mac sleeping when you plug in the charger with the lid closed (see below) |
| Launch at login | Starts Keepy Uppy when you log in, disarmed |
| Check for updates | Looks up the latest GitHub release. The app only goes online when you click this |

### The charger chime

If you plug in the charger while the lid is closed, macOS plays its "connected" chime. The chime holds a power assertion for about a second. When it ends, macOS re-evaluates lid-close sleep and the Mac sleeps about 14 ms later, faster than any app can undo it. Mute charger chime sets `com.apple.PowerChime ChimeOnNoHardware`, so the chime and its assertion never happen. Untick it to restore the macOS default. Unplugging the charger doesn't trigger this.

## Safety

A Mac that can't sleep with its lid closed can overheat in a bag, so Keepy Uppy turns itself off whenever it can't be sure you still want it on.

- It always starts disarmed. Arming is always your choice, and the app never restores the armed state at launch.
- Quit, Ctrl-C and `kill` all disarm before the app exits.
- If the app crashes or is force-quit (including `kill -9`), a small watchdog process notices and turns lid-close sleep back on.
- On battery, it disarms at the low-battery threshold. It also disarms when macOS reports serious or critical heat.
- Timers end on time. Any automatic disarm ends the session, and you have to re-arm yourself.

Settings are stored in `~/Library/Application Support/keepy-uppy/config.toml`.

## How it works

macOS sleeps on lid close unless a kernel flag tells it not to. Keepy Uppy sets that flag with the same private power-management call that macOS's own power daemon uses (`IOPMrootDomain`, selector `kPMSetClamshellSleepState`). The call needs neither root nor an entitlement, so the app installs no privileged helper and never touches `pmset disablesleep`.

`powerd` shares the flag and sometimes clears it, so the app sets it again every second while armed. The kernel also keeps the flag after the app exits. That's why the app disarms at launch and runs the crash watchdog.

[docs/mechanism.md](docs/mechanism.md) has the full details, the on-device test results and the known limits.

### Limits

- It relies on a private macOS interface, which a future macOS release could change or remove.
- Plugging in an external display while the lid is closed hasn't been tested.
- Only Apple Silicon builds exist for now.

## Building from source

You need stable Rust and the Xcode command line tools.

```sh
cargo run                      # run from the terminal
cargo test                     # policy, config, update and icon tests
sh scripts/bundle.sh           # build "target/release/Keepy Uppy.app" (ad-hoc signed)
```

To build a release DMG that is signed with a Developer ID, notarized and stapled:

```sh
DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)" \
NOTARY_PROFILE=keepy-uppy \
sh scripts/release.sh
```

The comments at the top of `scripts/release.sh` explain the one-time `notarytool` setup.

The icons are drawn in code. `scripts/draw-menubar-icons.swift` draws the menu bar icons and `scripts/build-app-icon.sh` builds the app icon.

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

[GPL-3.0](LICENSE). If you distribute a modified version, you have to release its source under the same license.
