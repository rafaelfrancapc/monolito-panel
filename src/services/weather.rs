//! Lightweight weather provider. Empty location lets wttr.in infer it from the public IP.
use super::system;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weather {
    pub temperature: String,
    pub condition: String,
    pub icon: &'static str,
}

fn encode_location(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
                vec![byte as char]
            } else if byte == b' ' {
                vec!['+']
            } else {
                format!("%{byte:02X}").chars().collect()
            }
        })
        .collect()
}

fn icon_for(condition: &str) -> &'static str {
    let value = condition.to_ascii_lowercase();
    if value.contains("thunder") || value.contains("storm") {
        "weather-storm-symbolic"
    } else if value.contains("snow") || value.contains("sleet") || value.contains("blizzard") {
        "weather-snow-symbolic"
    } else if value.contains("rain") || value.contains("drizzle") || value.contains("shower") {
        "weather-showers-symbolic"
    } else if value.contains("fog") || value.contains("mist") || value.contains("haze") {
        "weather-fog-symbolic"
    } else if value.contains("cloud") || value.contains("overcast") {
        "weather-overcast-symbolic"
    } else {
        "weather-clear-symbolic"
    }
}

pub fn parse(value: &str) -> Result<Weather, String> {
    let mut fields = value.trim().splitn(2, '|');
    let temperature = fields.next().unwrap_or_default().trim();
    let condition = fields.next().unwrap_or_default().trim();
    if temperature.is_empty() || condition.is_empty() {
        return Err("Weather provider returned an incomplete response".into());
    }
    Ok(Weather {
        temperature: temperature.replace('+', ""),
        condition: condition.into(),
        icon: icon_for(condition),
    })
}

pub async fn current(location: Option<&str>) -> Result<Weather, String> {
    let location = location
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(encode_location)
        .unwrap_or_default();
    let url = format!("https://wttr.in/{location}?format=%25t%7C%25C&m");
    let response = system::command(&[
        "curl",
        "--fail",
        "--silent",
        "--show-error",
        "--max-time",
        "12",
        "--user-agent",
        "monolito-panel/0.1.3",
        &url,
    ])
    .await?;
    parse(&response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_conditions_and_encodes_custom_location() {
        let weather = parse("+23°C|Partly cloudy").unwrap();
        assert_eq!(weather.temperature, "23°C");
        assert_eq!(weather.icon, "weather-overcast-symbolic");
        assert_eq!(encode_location("São Paulo"), "S%C3%A3o+Paulo");
    }
}
