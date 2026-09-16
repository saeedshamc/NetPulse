use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub poll_interval_secs: u64,
    pub theme: String,
    pub language: String,
    pub start_with_os: bool,
    pub close_to_tray: bool,
    pub retention_days: u32,
    pub alert_enabled: bool,
    pub alert_daily_bytes: u64,
    pub alert_ssid_only: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            poll_interval_secs: 2,
            theme: "dark".into(),
            language: "en".into(),
            start_with_os: false,
            close_to_tray: true,
            retention_days: 90,
            alert_enabled: false,
            alert_daily_bytes: 5_368_709_120, // 5 GiB
            alert_ssid_only: None,
        }
    }
}

impl AppSettings {
    pub fn path_in(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    pub fn load(dir: &Path) -> Self {
        let path = Self::path_in(dir);
        let Ok(raw) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = Self::path_in(dir);
        let raw = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, raw).map_err(|e| e.to_string())
    }

    pub fn sanitize(mut self) -> Self {
        if self.poll_interval_secs < 1 {
            self.poll_interval_secs = 1;
        }
        if self.poll_interval_secs > 60 {
            self.poll_interval_secs = 60;
        }
        if self.retention_days < 7 {
            self.retention_days = 7;
        }
        if self.theme != "light" && self.theme != "dark" {
            self.theme = "dark".into();
        }
        if self.language != "fa" && self.language != "en" {
            self.language = "en".into();
        }
        self
    }
}
