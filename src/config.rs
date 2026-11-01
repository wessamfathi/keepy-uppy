//! User settings, stored as `config.toml` in the platform config dir.
//! The armed state is deliberately not stored: the app always starts disarmed.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::icons::IconStyle;
use crate::platform;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Disarm on battery at or below this percentage. 0 = never.
    /// (TOML has no null, so `None` is stored as 0.)
    low_battery_pct: u8,
    pub icon_style: IconStyle,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            low_battery_pct: 10,
            icon_style: IconStyle::default(),
        }
    }
}

impl Config {
    pub fn low_battery(&self) -> Option<u8> {
        (self.low_battery_pct > 0).then_some(self.low_battery_pct)
    }

    pub fn set_low_battery(&mut self, limit: Option<u8>) {
        self.low_battery_pct = limit.unwrap_or(0);
    }

    /// Missing or unreadable file → defaults.
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = path().ok_or("no config directory")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("config: {e}"))?;
        }
        let text = toml::to_string(self).map_err(|e| format!("config: {e}"))?;
        std::fs::write(&path, text).map_err(|e| format!("config: {e}"))
    }
}

fn path() -> Option<PathBuf> {
    Some(platform::config_dir()?.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn never_round_trips() {
        let mut c = Config::default();
        c.set_low_battery(None);
        let back: Config = toml::from_str(&toml::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.low_battery(), None);
    }

    #[test]
    fn missing_key_uses_default() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c.low_battery(), Some(10));
    }
}
