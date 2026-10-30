//! Pure arm/disarm rules. No OS calls, so every rule is unit-tested.

use std::time::Instant;

/// What the user asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Session {
    Off,
    Until(Instant),
    Indefinite,
}

/// Mirrors `NSProcessInfoThermalState`.
#[allow(dead_code)] // TODO: read from NSProcessInfo (plan step 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thermal {
    Nominal,
    Fair,
    Serious,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Power {
    pub on_ac: bool,
    /// `None` when the Mac has no battery.
    pub battery_pct: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct Inputs {
    pub session: Session,
    pub now: Instant,
    pub power: Power,
    /// Disarm on battery at or below this percentage. `None` = never.
    pub low_battery: Option<u8>,
    pub thermal: Thermal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Arm,
    Disarm(Reason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The user has not armed.
    Off,
    Expired,
    LowBattery,
    Hot,
}

pub fn decide(i: &Inputs) -> Decision {
    match i.session {
        Session::Off => return Decision::Disarm(Reason::Off),
        Session::Until(end) if i.now >= end => return Decision::Disarm(Reason::Expired),
        _ => {}
    }
    if matches!(i.thermal, Thermal::Serious | Thermal::Critical) {
        return Decision::Disarm(Reason::Hot);
    }
    if !i.power.on_ac {
        if let (Some(limit), Some(pct)) = (i.low_battery, i.power.battery_pct) {
            if pct <= limit {
                return Decision::Disarm(Reason::LowBattery);
            }
        }
    }
    Decision::Arm
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn inputs(session: Session) -> Inputs {
        Inputs {
            session,
            now: Instant::now(),
            power: Power {
                on_ac: false,
                battery_pct: Some(50),
            },
            low_battery: Some(10),
            thermal: Thermal::Nominal,
        }
    }

    #[test]
    fn off_disarms() {
        assert_eq!(decide(&inputs(Session::Off)), Decision::Disarm(Reason::Off));
    }

    #[test]
    fn indefinite_arms() {
        assert_eq!(decide(&inputs(Session::Indefinite)), Decision::Arm);
    }

    #[test]
    fn timer_arms_until_expiry() {
        let mut i = inputs(Session::Indefinite);
        i.session = Session::Until(i.now + Duration::from_secs(60));
        assert_eq!(decide(&i), Decision::Arm);
        i.now += Duration::from_secs(60);
        assert_eq!(decide(&i), Decision::Disarm(Reason::Expired));
    }

    #[test]
    fn low_battery_disarms_at_threshold() {
        let mut i = inputs(Session::Indefinite);
        i.power.battery_pct = Some(11);
        assert_eq!(decide(&i), Decision::Arm);
        i.power.battery_pct = Some(10);
        assert_eq!(decide(&i), Decision::Disarm(Reason::LowBattery));
    }

    #[test]
    fn low_battery_ignored_on_ac() {
        let mut i = inputs(Session::Indefinite);
        i.power = Power {
            on_ac: true,
            battery_pct: Some(3),
        };
        assert_eq!(decide(&i), Decision::Arm);
    }

    #[test]
    fn low_battery_threshold_off() {
        let mut i = inputs(Session::Indefinite);
        i.power.battery_pct = Some(1);
        i.low_battery = None;
        assert_eq!(decide(&i), Decision::Arm);
    }

    #[test]
    fn no_battery_arms() {
        let mut i = inputs(Session::Indefinite);
        i.power.battery_pct = None;
        assert_eq!(decide(&i), Decision::Arm);
    }

    #[test]
    fn hot_disarms() {
        let mut i = inputs(Session::Indefinite);
        i.thermal = Thermal::Fair;
        assert_eq!(decide(&i), Decision::Arm);
        for t in [Thermal::Serious, Thermal::Critical] {
            i.thermal = t;
            assert_eq!(decide(&i), Decision::Disarm(Reason::Hot));
        }
    }

    #[test]
    fn expiry_wins_over_other_reasons() {
        let mut i = inputs(Session::Until(Instant::now()));
        i.thermal = Thermal::Critical;
        assert_eq!(decide(&i), Decision::Disarm(Reason::Expired));
    }
}
