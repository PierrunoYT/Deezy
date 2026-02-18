use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub arl: String,
    pub output_dir: String,
    pub quality: String,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());

        let default_dir = PathBuf::from(home)
            .join("Music")
            .join("Deezy");

        Self {
            arl: String::new(),
            output_dir: default_dir.to_string_lossy().to_string(),
            quality: "MP3_320".into(),
        }
    }
}

impl Settings {
    fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir.join("settings.json"))
    }

    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let path = Self::path(app)?;
        if path.exists() {
            let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&data).map_err(|e| e.to_string())
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let path = Self::path(app)?;
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, data).map_err(|e| e.to_string())
    }
}
