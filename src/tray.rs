//! Menu bar icon and menu. Holds no state of its own: `render` mirrors `View`.

use std::time::{Duration, Instant};

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::icons::IconStyle;
use crate::platform;
use crate::policy::{Reason, Session};
use crate::update::UpdateState;

pub const LOW_BATTERY_CHOICES: [Option<u8>; 4] = [Some(10), Some(20), Some(30), None];

pub enum Action {
    /// Hours; `None` = until disarmed.
    Arm(Option<u64>),
    Disarm,
    LowBattery(Option<u8>),
    IconStyle(IconStyle),
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
    pub icon_style: IconStyle,
    pub chime_muted: bool,
    pub launch_at_login: bool,
    pub update: &'a UpdateState,
    pub error: Option<&'a str>,
}

pub struct Tray {
    icon: TrayIcon,
    /// What the icon currently shows, to skip redundant redraws.
    shown: Option<(IconStyle, bool)>,
    status: MenuItem,
    indefinite: CheckMenuItem,
    hour1: CheckMenuItem,
    hour4: CheckMenuItem,
    disarm: MenuItem,
    low_battery: Vec<(Option<u8>, CheckMenuItem)>,
    icon_styles: Vec<(IconStyle, CheckMenuItem)>,
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
        let icon_styles: Vec<_> = IconStyle::ALL
            .iter()
            .map(|&s| (s, CheckMenuItem::new(s.label(), true, false, None)))
            .collect();
        let icon_menu = Submenu::new("Icon", true);
        for (_, item) in &icon_styles {
            icon_menu.append(item).map_err(|e| e.to_string())?;
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
            &icon_menu,
            &PredefinedMenuItem::separator(),
            &update,
            &quit,
        ])
        .map_err(|e| e.to_string())?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Keepy Uppy")
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            icon,
            shown: None,
            status,
            indefinite,
            hour1,
            hour4,
            disarm,
            low_battery,
            icon_styles,
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
                .or_else(|| {
                    self.icon_styles
                        .iter()
                        .find(|(_, item)| id == item.id())
                        .map(|&(s, _)| Action::IconStyle(s))
                })
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

        for (s, item) in &self.icon_styles {
            item.set_checked(*s == v.icon_style);
        }

        let wanted = (v.icon_style, v.armed);
        if self.shown != Some(wanted) {
            let _ = self
                .icon
                .set_icon_templated(Some(v.icon_style.icon(v.armed)));
            platform::fit_status_icon(&self.icon);
            self.shown = Some(wanted);
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
