use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

const KEYRING_SERVICE: &str = "com.pierr.deezy";
const KEYRING_USER: &str = "arl_token";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FolderStructure {
    #[default]
    Flat,
    ArtistTrack,
    ArtistAlbumTrack,
    AlbumTrack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub arl: String,
    pub output_dir: String,
    pub quality: String,
    #[serde(default)]
    pub folder_structure: FolderStructure,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub custom_theme: Option<String>,
    #[serde(default)]
    pub search_history: Vec<String>,
    #[serde(default = "default_true")]
    pub enable_search_history: bool,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
}

fn default_true() -> bool {
    true
}

fn default_locale() -> String {
    "en".to_string()
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
            folder_structure: FolderStructure::default(),
            theme: Some("system".to_string()),
            custom_theme: None,
            search_history: Vec::new(),
            enable_search_history: true,
            notifications_enabled: true,
            locale: "en".to_string(),
            close_to_tray: true,
        }
    }
}

fn save_arl_to_keyring(arl: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keyring error: {}", e))?;
    entry
        .set_password(arl)
        .map_err(|e| format!("Failed to save ARL to credential store: {}", e))
}

fn load_arl_from_keyring() -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    entry.get_password().ok()
}

impl Settings {
    fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir.join("settings.json"))
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate ARL
        if self.arl.trim().is_empty() {
            return Err("ARL token is required".to_string());
        }

        if self.arl.trim().len() < 100 {
            return Err("ARL token appears to be invalid (too short)".to_string());
        }

        // Validate output directory
        if self.output_dir.trim().is_empty() {
            return Err("Output directory is required".to_string());
        }

        // Try to create the directory if it doesn't exist
        let output_path = PathBuf::from(&self.output_dir);
        if !output_path.exists() {
            std::fs::create_dir_all(&output_path)
                .map_err(|e| format!("Cannot create output directory: {}", e))?;
        }

        // Check if directory is writable
        if !output_path.is_dir() {
            return Err("Output path is not a directory".to_string());
        }

        // Validate quality
        let valid_qualities = ["MP3_128", "MP3_320", "FLAC"];
        if !valid_qualities.contains(&self.quality.as_str()) {
            return Err(format!("Invalid quality '{}'. Must be one of: MP3_128, MP3_320, FLAC", self.quality));
        }

        Ok(())
    }

    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let path = Self::path(app)?;
        let mut settings: Self = if path.exists() {
            let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&data).map_err(|e| e.to_string())?
        } else {
            Self::default()
        };

        // Migrate: if ARL was stored in the JSON file, move it to keyring
        if !settings.arl.is_empty() {
            save_arl_to_keyring(&settings.arl)?;
            // Re-save settings without the ARL in the file
            let mut clean = settings.clone();
            clean.arl = String::new();
            let data = serde_json::to_string_pretty(&clean).map_err(|e| e.to_string())?;
            std::fs::write(&path, &data).map_err(|e| e.to_string())?;
        }

        // Load ARL from OS credential store
        if let Some(arl) = load_arl_from_keyring() {
            if !arl.is_empty() {
                settings.arl = arl;
            }
        }

        Ok(settings)
    }

    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        // Validate before saving
        self.validate()?;

        // Save ARL to OS credential store.
        // Do not persist plaintext credentials to disk if credential storage fails.
        save_arl_to_keyring(&self.arl)?;

        // Write settings to disk without ARL.
        let mut settings_for_disk = self.clone();
        settings_for_disk.arl = String::new();

        let path = Self::path(app)?;
        let data = serde_json::to_string_pretty(&settings_for_disk).map_err(|e| e.to_string())?;
        std::fs::write(&path, &data).map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}
