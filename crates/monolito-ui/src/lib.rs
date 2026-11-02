//! Shared appearance contract. Independent of the Settings process and GTK.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CSS: &str = include_str!("../../../resources/styles/monolito.css");
pub const START_WIDTH: i32 = 552;
pub const QUICK_WIDTH: i32 = 352;
pub const FLYOUT_PADDING: i32 = 24;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Appearance {
    pub version: u32,
    pub theme: Theme,
    pub accent: String,
    pub animations: bool,
    /// Installed GTK icon theme; the standard Win11 icon pack is expected.
    #[serde(default = "default_icon_theme")]
    pub icon_theme: Option<String>,
}

fn default_icon_theme() -> Option<String> {
    Some("Win11".into())
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
                    accent: "#0078d4".into(),
                    animations: true,
                    icon_theme: default_icon_theme(),
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
            "#0078d4"
        };
        let colors = if self.theme == Theme::Dark {
            [
                "#202020", "#2c2c2c", "#3a3a3a", "#454545", "#ffffff", "#d6d6d6", "#606060",
                "#ffffff", "#1f1f1f", "#9c9c9c", "#9c9c9c",
            ]
        } else {
            [
                "#f3f3f3", "#f9f9f9", "#ffffff", "#ededed", "#1a1a1a", "#515151", "#b8b8b8",
                "#ffffff", "#ffffff", "#737373", "#737373",
            ]
        };
        let mut css = String::new();
        for (name, color) in [
            "canvas",
            "primary",
            "secondary",
            "subtle",
            "text",
            "muted",
            "border",
            "on_accent",
            "input",
            "track",
            "disabled",
        ]
        .iter()
        .zip(colors)
        {
            css.push_str(&format!("@define-color {name} {color};\n"));
        }
        // Select foreground by measured contrast, including custom accent colors.
        let on_accent = if contrast(accent, "#ffffff") >= contrast(accent, "#1a1a1a") {
            "#ffffff"
        } else {
            "#1a1a1a"
        };
        css.push_str(&format!(
            "@define-color accent {accent};\n@define-color on_accent {on_accent};"
        ));
        css
    }
}

pub fn contrast(a: &str, b: &str) -> f64 {
    fn luminance(hex: &str) -> f64 {
        [0.2126, 0.7152, 0.0722]
            .iter()
            .enumerate()
            .map(|(i, weight)| {
                let channel =
                    u8::from_str_radix(&hex[1 + i * 2..3 + i * 2], 16).unwrap() as f64 / 255.0;
                weight
                    * if channel <= 0.04045 {
                        channel / 12.92
                    } else {
                        ((channel + 0.055) / 1.055).powf(2.4)
                    }
            })
            .sum()
    }
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
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

    #[test]
    fn win11_icons_are_the_default_in_existing_appearance_files() {
        let appearance: Appearance =
            toml::from_str("version = 1\ntheme = 'dark'\naccent = '#0078d4'\nanimations = true\n")
                .unwrap();
        assert_eq!(appearance.icon_theme.as_deref(), Some("Win11"));
    }

    #[test]
    fn fluent_text_and_controls_meet_contrast_targets() {
        for (fg, bg) in [
            ("#ffffff", "#2c2c2c"),
            ("#d6d6d6", "#3a3a3a"),
            ("#1a1a1a", "#f9f9f9"),
            ("#515151", "#ededed"),
            ("#ffffff", "#0078d4"),
        ] {
            assert!(contrast(fg, bg) >= 4.5, "{fg} on {bg}");
        }
        assert!(contrast("#9c9c9c", "#2c2c2c") >= 3.0);
        assert!(contrast("#737373", "#f9f9f9") >= 3.0);
    }
}
