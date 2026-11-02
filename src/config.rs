use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskbarAlignment {
    Left,
    #[default]
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub height: u16,
    /// Index in RandR's monitor list; None selects the primary monitor.
    pub monitor: Option<usize>,
    pub pinned: Vec<String>,
    pub start_pinned: Vec<String>,
    pub weather_enabled: bool,
    /// Empty uses the provider's IP-based location; otherwise accepts a city name.
    pub weather_location: Option<String>,
    pub alignment: TaskbarAlignment,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            height: 48,
            monitor: None,
            pinned: vec![],
            start_pinned: vec![],
            weather_enabled: true,
            weather_location: None,
            alignment: TaskbarAlignment::Center,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        monolito_ui::config_dir().join("panel.toml")
    }

    pub fn load() -> Result<Self> {
        match std::fs::read_to_string(Self::path()) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut value: Self = toml::from_str(text)?;
        if value.version != 1 {
            bail!("Unsupported panel configuration version {}", value.version);
        }
        if !(32..=96).contains(&value.height) {
            bail!("Panel height must be between 32 and 96");
        }
        value.start_pinned.retain(|id| !value.pinned.contains(id));
        Ok(value)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .context("Configuration has no parent directory")?;
        std::fs::create_dir_all(parent)?;
        let temp = path.with_extension(format!("toml.{}.tmp", std::process::id()));
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(toml::to_string_pretty(self)?.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok(())
    }

    pub fn toggle_pin(&mut self, id: &str) {
        if self.pinned.iter().any(|p| p == id) {
            self.pinned.retain(|p| p != id);
        } else {
            self.start_pinned.retain(|p| p != id);
            self.pinned.push(id.to_owned());
        }
    }
    pub fn toggle_start_pin(&mut self, id: &str) {
        if self.start_pinned.iter().any(|p| p == id) {
            self.start_pinned.retain(|p| p != id);
        } else {
            self.pinned.retain(|p| p != id);
            self.start_pinned.push(id.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_height_and_future_schema() {
        assert!(Config::parse("height = 12").is_err());
        assert!(Config::parse("version = 99").is_err());
        assert!(Config::parse("height = 64").is_ok());
    }

    #[test]
    fn parses_alignment_option() {
        let conf = Config::parse("alignment = 'left'").unwrap();
        assert_eq!(conf.alignment, TaskbarAlignment::Left);
        let conf_center = Config::parse("alignment = 'center'").unwrap();
        assert_eq!(conf_center.alignment, TaskbarAlignment::Center);
    }

    #[test]
    fn pins_roundtrip_in_order() {
        let mut config = Config::default();
        config.toggle_pin("org.xfce.Thunar.desktop");
        config.toggle_pin("firefox.desktop");
        config.toggle_start_pin("terminal.desktop");
        assert!(!config.pinned.contains(&"terminal.desktop".into()));
        config.toggle_start_pin("firefox.desktop");
        assert!(!config.pinned.contains(&"firefox.desktop".into()));
        config.toggle_pin("firefox.desktop");
        assert!(!config.start_pinned.contains(&"firefox.desktop".into()));
        let path =
            std::env::temp_dir().join(format!("monolito-config-test-{}.toml", std::process::id()));
        config.save_to(&path).unwrap();
        assert_eq!(
            Config::parse(&std::fs::read_to_string(&path).unwrap()).unwrap(),
            config
        );
        config.toggle_pin("org.xfce.Thunar.desktop");
        assert_eq!(config.pinned, ["firefox.desktop"]);
        std::fs::remove_file(path).unwrap();
    }
}
