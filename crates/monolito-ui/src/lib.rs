//! Shared appearance contract. Independent of the Settings process and GTK.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CSS: &str = include_str!("../../../resources/styles/monolito.css");

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Appearance {
    pub version: u32,
    pub theme: Theme,
    pub accent: String,
    pub animations: bool,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("monolito")
}

impl Appearance {
    pub fn load() -> Result<Self, String> {
        let path = config_dir().join("appearance.toml");
        let value = match std::fs::read_to_string(&path) {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    version: 1,
                    accent: "#49b9ed".into(),
                    animations: true,
                    ..Self::default()
                });
            }
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let appearance: Self = toml::from_str(&value).map_err(|e| e.to_string())?;
        if appearance.version != 1 {
            return Err("Unsupported appearance version".into());
        }
        if !valid_accent(&appearance.accent) {
            return Err("Accent must be #RRGGBB".into());
        }
        Ok(appearance)
    }

    pub fn accent_css(&self) -> String {
        let accent = if valid_accent(&self.accent) {
            &self.accent
        } else {
            "#49b9ed"
        };
        format!("@define-color monolito_accent {accent};")
    }
}

pub fn valid_accent(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accent_cannot_inject_css() {
        assert!(valid_accent("#39aBeF"));
        assert!(!valid_accent("red; } * { color: red"));
        assert!(!valid_accent("#fff"));
    }
}
