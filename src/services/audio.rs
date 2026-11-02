//! Audio management via PipeWire/WirePlumber (wpctl) and PulseAudio fallback.
use super::system;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioVolume {
    pub value: f64, // 0.0 to 1.0
    pub muted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    pub id: u32,
    pub description: String,
    pub is_default: bool,
}

pub async fn get_master_volume() -> Result<AudioVolume, String> {
    let out = system::command(&["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]).await?;
    let (value, muted) =
        system::parse_volume(&out).ok_or_else(|| "Failed to parse sink volume".to_string())?;
    Ok(AudioVolume { value, muted })
}

pub async fn set_master_volume(value: f64) -> Result<(), String> {
    let clamped = value.clamp(0.0, 1.0);
    system::command(&[
        "wpctl",
        "set-volume",
        "-l",
        "1.0",
        "@DEFAULT_AUDIO_SINK@",
        &format!("{clamped:.2}"),
    ])
    .await?;
    Ok(())
}

pub async fn toggle_master_mute() -> Result<(), String> {
    system::command(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).await?;
    Ok(())
}

pub async fn get_input_volume() -> Result<AudioVolume, String> {
    let out = system::command(&["wpctl", "get-volume", "@DEFAULT_AUDIO_SOURCE@"]).await?;
    let (value, muted) =
        system::parse_volume(&out).ok_or_else(|| "Failed to parse source volume".to_string())?;
    Ok(AudioVolume { value, muted })
}

pub async fn set_input_volume(value: f64) -> Result<(), String> {
    let clamped = value.clamp(0.0, 1.0);
    system::command(&[
        "wpctl",
        "set-volume",
        "-l",
        "1.0",
        "@DEFAULT_AUDIO_SOURCE@",
        &format!("{clamped:.2}"),
    ])
    .await?;
    Ok(())
}

pub async fn toggle_input_mute() -> Result<(), String> {
    system::command(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"]).await?;
    Ok(())
}

pub async fn list_output_devices() -> Result<Vec<AudioDevice>, String> {
    let status_output = system::command(&["wpctl", "status"]).await?;
    Ok(parse_wpctl_sinks(&status_output))
}

pub async fn set_default_output_device(id: u32) -> Result<(), String> {
    system::command(&["wpctl", "set-default", &id.to_string()]).await?;
    Ok(())
}

/// Parses the "Sinks:" section of `wpctl status` output
pub fn parse_wpctl_sinks(output: &str) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    let mut in_sinks = false;

    for line in output.lines() {
        let trimmed = line.trim();
        let section = trimmed.trim_start_matches(['│', '├', '└', '─', ' ']);
        if section == "Sinks:" || trimmed.contains("Audio/Sink") {
            in_sinks = true;
            continue;
        }

        if in_sinks {
            // End of Sinks section when another heading begins
            if section.ends_with(':') && !section.starts_with('*') {
                break;
            }

            // Line format in wpctl status:
            // " │  *   48. Built-in Audio Analog Stereo        [vol: 0.65]"
            // or " │      49. HDMI / DisplayPort 1 Output         [vol: 1.00]"
            let is_default = line.contains('*');
            let clean = line.replace(['│', '*'], " ");

            let parts: Vec<&str> = clean.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Look for "<id>." pattern
            if let Some(first) = parts.first() {
                if let Some(id_str) = first.strip_suffix('.') {
                    if let Ok(id) = id_str.parse::<u32>() {
                        // Gather description until [vol: ...] or [MUTED]
                        let mut desc_words = Vec::new();
                        for word in &parts[1..] {
                            if word.starts_with('[') {
                                break;
                            }
                            desc_words.push(*word);
                        }
                        let description = desc_words.join(" ");
                        if !description.is_empty() {
                            devices.push(AudioDevice {
                                id,
                                description,
                                is_default,
                            });
                        }
                    }
                }
            }
        }
    }

    devices
}

pub fn volume_icon(volume: f64, muted: bool) -> &'static str {
    if muted || volume <= 0.001 {
        "audio-volume-muted-symbolic"
    } else if volume < 0.34 {
        "audio-volume-low-symbolic"
    } else if volume < 0.67 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    }
}

pub fn input_icon(muted: bool) -> &'static str {
    if muted {
        "microphone-sensitivity-muted-symbolic"
    } else {
        "audio-input-microphone-symbolic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_icon_transitions_correctly() {
        assert_eq!(volume_icon(0.0, false), "audio-volume-muted-symbolic");
        assert_eq!(volume_icon(0.5, true), "audio-volume-muted-symbolic");
        assert_eq!(volume_icon(0.2, false), "audio-volume-low-symbolic");
        assert_eq!(volume_icon(0.5, false), "audio-volume-medium-symbolic");
        assert_eq!(volume_icon(0.9, false), "audio-volume-high-symbolic");
    }

    #[test]
    fn parses_wpctl_sinks_correctly() {
        let sample = r#"
PipeWire 'pipewire-0' [0.3.79, user, core]
 └─ Clients:
Audio
 ├─ Devices:
 ├─ Sinks:
 │  *   48. Built-in Audio Analog Stereo        [vol: 0.65]
 │      49. USB Audio DAC                       [vol: 0.40 MUTED]
 │      50. HDMI Output                         [vol: 1.00]
 ├─ Sources:
 │  *   51. Built-in Audio Analog Stereo        [vol: 0.50]
"#;
        let devices = parse_wpctl_sinks(sample);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].id, 48);
        assert_eq!(devices[0].description, "Built-in Audio Analog Stereo");
        assert!(devices[0].is_default);

        assert_eq!(devices[1].id, 49);
        assert_eq!(devices[1].description, "USB Audio DAC");
        assert!(!devices[1].is_default);

        assert_eq!(devices[2].id, 50);
        assert_eq!(devices[2].description, "HDMI Output");
        assert!(!devices[2].is_default);
    }
}
