//! Menu bar icon and menu. Holds no state of its own: `render` mirrors `View`.

use std::time::{Duration, Instant};

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::policy::{Reason, Session};
use crate::update::UpdateState;

pub const LOW_BATTERY_CHOICES: [Option<u8>; 4] = [Some(10), Some(20), Some(30), None];

pub enum Action {
    /// Hours; `None` = until disarmed.
    Arm(Option<u64>),
    Disarm,
    LowBattery(Option<u8>),
    ToggleChime,
    ToggleLaunchAtLogin,
    CheckForUpdates,
    Quit,
}

/// Everything the menu shows.
pub struct View<'a> {
    pub session: Session,
    /// Length of the timer the user picked, for the check mark.
    pub timer_hours: Option<u64>,
    pub armed: bool,
    pub now: Instant,
    pub last_reason: Reason,
    pub low_battery: Option<u8>,
    pub chime_muted: bool,
    pub launch_at_login: bool,
    pub update: &'a UpdateState,
    pub error: Option<&'a str>,
}

pub struct Tray {
    icon: TrayIcon,
    shown_armed: Option<bool>,
    status: MenuItem,
    indefinite: CheckMenuItem,
    hour1: CheckMenuItem,
    hour4: CheckMenuItem,
    disarm: MenuItem,
    low_battery: Vec<(Option<u8>, CheckMenuItem)>,
    chime: CheckMenuItem,
    launch_at_login: CheckMenuItem,
    update: MenuItem,
    quit: MenuItem,
}

impl Tray {
    /// Must run on the main thread after the event loop has started.
    pub fn new() -> Result<Self, String> {
        let status = MenuItem::new("", false, None);
        let indefinite = CheckMenuItem::new("Keep awake until disarmed", true, false, None);
        let hour1 = CheckMenuItem::new("Keep awake for 1 hour", true, false, None);
        let hour4 = CheckMenuItem::new("Keep awake for 4 hours", true, false, None);
        let disarm = MenuItem::new("Disarm", true, None);
        let low_battery: Vec<_> = LOW_BATTERY_CHOICES
            .iter()
            .map(|&c| {
                let label = match c {
                    Some(p) => format!("At {p}%"),
                    None => "Never".into(),
                };
                (c, CheckMenuItem::new(label, true, false, None))
            })
            .collect();
        let low_battery_menu = Submenu::new("Disarm on low battery", true);
        for (_, item) in &low_battery {
            low_battery_menu.append(item).map_err(|e| e.to_string())?;
        }
        let chime = CheckMenuItem::new(
            "Mute charger chime (keeps awake when plugging in)",
            true,
            false,
            None,
        );
        let launch_at_login = CheckMenuItem::new("Launch at login", true, false, None);
        let update = MenuItem::new("", true, None);
        let quit = MenuItem::new("Quit Keepy Uppy", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &indefinite,
            &hour1,
            &hour4,
            &disarm,
            &PredefinedMenuItem::separator(),
            &low_battery_menu,
            &chime,
            &launch_at_login,
            &PredefinedMenuItem::separator(),
            &update,
            &quit,
        ])
        .map_err(|e| e.to_string())?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon_templated(icon(false))
            .with_tooltip("Keepy Uppy")
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            icon,
            shown_armed: None,
            status,
            indefinite,
            hour1,
            hour4,
            disarm,
            low_battery,
            chime,
            launch_at_login,
            update,
            quit,
        })
    }

    pub fn action(&self, id: &MenuId) -> Option<Action> {
        if id == self.indefinite.id() {
            Some(Action::Arm(None))
        } else if id == self.hour1.id() {
            Some(Action::Arm(Some(1)))
        } else if id == self.hour4.id() {
            Some(Action::Arm(Some(4)))
        } else if id == self.disarm.id() {
            Some(Action::Disarm)
        } else if id == self.chime.id() {
            Some(Action::ToggleChime)
        } else if id == self.launch_at_login.id() {
            Some(Action::ToggleLaunchAtLogin)
        } else if id == self.update.id() {
            Some(Action::CheckForUpdates)
        } else if id == self.quit.id() {
            Some(Action::Quit)
        } else {
            self.low_battery
                .iter()
                .find(|(_, item)| id == item.id())
                .map(|&(c, _)| Action::LowBattery(c))
        }
    }

    /// Called after every change and every tick.
    pub fn render(&mut self, v: &View) {
        self.status.set_text(status_text(v));
        // Check items toggle themselves on click; always reset them from state.
        let timer = |h| matches!(v.session, Session::Until(_)) && v.timer_hours == Some(h);
        self.indefinite
            .set_checked(v.session == Session::Indefinite);
        self.hour1.set_checked(timer(1));
        self.hour4.set_checked(timer(4));
        self.disarm.set_enabled(v.session != Session::Off);
        for (c, item) in &self.low_battery {
            item.set_checked(*c == v.low_battery);
        }
        self.chime.set_checked(v.chime_muted);
        self.launch_at_login.set_checked(v.launch_at_login);
        self.update.set_text(match v.update {
            UpdateState::Idle => "Check for updates…".into(),
            UpdateState::Checking => "Checking for updates…".into(),
            UpdateState::UpToDate => "Up to date · check again".into(),
            UpdateState::Available(tag) => format!("Download Keepy Uppy {tag}…"),
            UpdateState::Failed => "Couldn't check for updates · try again".into(),
        });
        self.update.set_enabled(*v.update != UpdateState::Checking);

        if self.shown_armed != Some(v.armed) {
            let _ = self.icon.set_icon_templated(Some(icon(v.armed)));
            self.shown_armed = Some(v.armed);
        }
    }
}

fn status_text(v: &View) -> String {
    if let Some(e) = v.error {
        return format!("Error: {e}");
    }
    match v.session {
        Session::Indefinite => "Awake with lid closed".into(),
        Session::Until(end) => format!(
            "Awake with lid closed · {} left",
            format_left(end.saturating_duration_since(v.now))
        ),
        Session::Off => match v.last_reason {
            Reason::Off => "Off: lid close sleeps".into(),
            Reason::Expired => "Off: timer ended".into(),
            Reason::LowBattery => "Off: battery low".into(),
            Reason::Hot => "Off: Mac too hot".into(),
        },
    }
}

/// Rounds up to the minute so "0m left" never shows while still armed.
fn format_left(d: Duration) -> String {
    let mins = d.as_secs().div_ceil(60);
    match (mins / 60, mins % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// Placeholder template icon: filled circle when armed, ring when off.
fn icon(armed: bool) -> Icon {
    const SIZE: u32 = 36; // 18pt at 2x
    let c = (SIZE as f32 - 1.0) / 2.0;
    let (outer, inner) = (14.0, if armed { 0.0 } else { 10.0 });
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let r = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            // 1px anti-aliased edges on both radii.
            let a = (outer + 0.5 - r).clamp(0.0, 1.0) * (r - inner + 0.5).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[0, 0, 0, (a * 255.0) as u8]);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("valid icon buffer")
}

#[cfg(test)]
mod tests {
    use super::format_left;
    use std::time::Duration;

    #[test]
    fn format_left_rounds_up() {
        assert_eq!(format_left(Duration::from_secs(1)), "1m");
        assert_eq!(format_left(Duration::from_secs(3600)), "1h");
        assert_eq!(format_left(Duration::from_secs(3601)), "1h 1m");
        assert_eq!(format_left(Duration::from_secs(4 * 3600 - 60)), "3h 59m");
    }
}
