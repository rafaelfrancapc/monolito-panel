//! UPower battery monitoring and display brightness controls.
use super::system;
use gtk::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum BatteryState {
    Unknown,
    Charging,
    Discharging,
    Empty,
    FullyCharged,
    PendingCharge,
    PendingDischarge,
}

impl BatteryState {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => BatteryState::Charging,
            2 => BatteryState::Discharging,
            3 => BatteryState::Empty,
            4 => BatteryState::FullyCharged,
            5 => BatteryState::PendingCharge,
            6 => BatteryState::PendingDischarge,
            _ => BatteryState::Unknown,
        }
    }

    pub fn is_charging(&self) -> bool {
        matches!(self, BatteryState::Charging | BatteryState::PendingCharge)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BatteryInfo {
    pub present: bool,
    pub percentage: f64,
    pub state: BatteryState,
    pub seconds_remaining: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessInfo {
    pub supported: bool,
    pub percentage: u32,
}

pub async fn get_battery() -> Result<Option<BatteryInfo>, String> {
    let proxy = match system::proxy(
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/DisplayDevice",
        "org.freedesktop.UPower.Device",
    )
    .await
    {
        Ok(p) if p.name_owner().is_some() => p,
        _ => return Ok(None),
    };

    let present = system::property::<bool>(&proxy, "IsPresent").unwrap_or(false);
    let dev_type = system::property::<u32>(&proxy, "Type").unwrap_or(0);

    // Type 2 == Battery
    if !present || dev_type != 2 {
        return Ok(None);
    }

    let percentage = system::property::<f64>(&proxy, "Percentage").unwrap_or(100.0);
    let state_code = system::property::<u32>(&proxy, "State").unwrap_or(0);
    let state = BatteryState::from_u32(state_code);

    let seconds = if state.is_charging() {
        system::property::<i64>(&proxy, "TimeToFull").filter(|&s| s > 0)
    } else {
        system::property::<i64>(&proxy, "TimeToEmpty").filter(|&s| s > 0)
    };

    Ok(Some(BatteryInfo {
        present: true,
        percentage,
        state,
        seconds_remaining: seconds,
    }))
}

pub fn battery_icon(percentage: f64, state: &BatteryState) -> &'static str {
    if state.is_charging() {
        return match percentage.round() as i32 {
            0..=15 => "battery-caution-charging-symbolic",
            16..=35 => "battery-low-charging-symbolic",
            36..=80 => "battery-good-charging-symbolic",
            _ => "battery-full-charging-symbolic",
        };
    }
    if *state == BatteryState::FullyCharged {
        return "battery-level-100-charged-symbolic";
    }
    match percentage.round() as i32 {
        0..=15 => "battery-level-10-symbolic",
        16..=25 => "battery-level-20-symbolic",
        26..=35 => "battery-level-30-symbolic",
        36..=45 => "battery-level-40-symbolic",
        46..=55 => "battery-level-50-symbolic",
        56..=65 => "battery-level-60-symbolic",
        66..=75 => "battery-level-70-symbolic",
        76..=85 => "battery-level-80-symbolic",
        86..=95 => "battery-level-90-symbolic",
        _ => "battery-level-100-symbolic",
    }
}

pub fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

pub async fn get_brightness() -> Result<BrightnessInfo, String> {
    // Try brightnessctl first
    if let (Ok(curr_out), Ok(max_out)) = (
        system::command(&["brightnessctl", "get"]).await,
        system::command(&["brightnessctl", "max"]).await,
    ) {
        if let (Ok(curr), Ok(max)) = (
            curr_out.trim().parse::<f64>(),
            max_out.trim().parse::<f64>(),
        ) {
            if max > 0.0 {
                let pct = ((curr / max) * 100.0).round() as u32;
                return Ok(BrightnessInfo {
                    supported: true,
                    percentage: pct.min(100),
                });
            }
        }
    }

    // Fallback: check xbacklight -get
    if let Ok(out) = system::command(&["xbacklight", "-get"]).await {
        if let Ok(val) = out.trim().parse::<f64>() {
            return Ok(BrightnessInfo {
                supported: true,
                percentage: (val.round() as u32).min(100),
            });
        }
    }

    Ok(BrightnessInfo {
        supported: false,
        percentage: 100,
    })
}

pub async fn set_brightness(percentage: u32) -> Result<(), String> {
    let clamped = percentage.min(100);
    if system::command(&["brightnessctl", "set", &format!("{clamped}%")])
        .await
        .is_ok()
    {
        return Ok(());
    }
    if system::command(&["xbacklight", "-set", &clamped.to_string()])
        .await
        .is_ok()
    {
        return Ok(());
    }
    Err("No supported brightness controller found".into())
}

pub async fn power_saver_enabled() -> Result<bool, String> {
    let profile = system::command(&["powerprofilesctl", "get"]).await?;
    Ok(profile.trim() == "power-saver")
}

pub async fn set_power_saver(enabled: bool) -> Result<(), String> {
    let profile = if enabled { "power-saver" } else { "balanced" };
    system::command(&["powerprofilesctl", "set", profile]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_battery_icons_correctly() {
        assert_eq!(
            battery_icon(99.0, &BatteryState::Discharging),
            "battery-level-100-symbolic"
        );
        assert_eq!(
            battery_icon(50.0, &BatteryState::Discharging),
            "battery-level-50-symbolic"
        );
        assert_eq!(
            battery_icon(10.0, &BatteryState::Discharging),
            "battery-level-10-symbolic"
        );
        assert_eq!(
            battery_icon(50.0, &BatteryState::Charging),
            "battery-good-charging-symbolic"
        );
    }

    #[test]
    fn formats_duration_cleanly() {
        assert_eq!(format_duration(3660), "1h 01m");
        assert_eq!(format_duration(7200), "2h 00m");
        assert_eq!(format_duration(1800), "30m");
    }
}
