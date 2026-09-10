use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::Manager;

const KEYRING_SERVICE: &str = "com.pierr.deezy";
const KEYRING_USER: &str = "arl_token";
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FolderStructure {
    #[default]
    Flat,
    ArtistTrack,
    ArtistAlbumTrack,
    AlbumTrack,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub arl: String,
    pub output_dir: String,
    pub quality: String,
    #[serde(default)]
    pub folder_structure: FolderStructure,
    #[serde(default = "default_custom_folder_template")]
    pub custom_folder_template: String,
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

fn default_custom_folder_template() -> String {
    "{artist}/{release_date} - {album}/{track_number} - {title}".to_string()
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
            custom_folder_template: default_custom_folder_template(),
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

fn keyring_enabled() -> bool {
    !matches!(
        std::env::var("DEEZY_NO_KEYRING").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn save_arl_to_keyring(arl: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keyring error: {}", e))?;
    entry
        .set_password(arl)
        .map_err(|e| format!("Failed to save ARL to credential store: {}", e))
}

fn delete_arl_from_keyring() -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keyring error: {}", e))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to remove stale ARL from credential store: {}", e)),
    }
}

fn load_arl_from_keyring() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keyring error: {}", e))?;
    match entry.get_password() {
        Ok(arl) => Ok(Some(arl)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read ARL from credential store: {}", e)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// The persisted location of the current ARL credential.
pub enum ArlStorage {
    Keyring,
    PlainFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The persisted ARL location and any storage warning detected while inspecting it.
pub struct ArlStorageStatus {
    pub storage: Option<ArlStorage>,
    pub reason: Option<String>,
}

impl ArlStorageStatus {
    fn none(reason: Option<String>) -> Self {
        Self { storage: None, reason }
    }
}

fn disk_arl(app: &tauri::AppHandle) -> Result<(Option<String>, Option<String>), String> {
    let path = Settings::path(app)?;
    let Some(contents) = read_private(&path)? else {
        return Ok((None, None));
    };
    let value: serde_json::Value =
        serde_json::from_str(&contents.data).map_err(|e| e.to_string())?;
    Ok((
        value.get("arl").and_then(|arl| arl.as_str()).map(str::to_string),
        contents.hardening_error,
    ))
}

/// Reports where the current ARL is actually persisted without exposing its value.
pub fn arl_storage_status(app: &tauri::AppHandle) -> ArlStorageStatus {
    let disk_hardening_error;
    match disk_arl(app) {
        Ok((Some(arl), hardening_error)) if !arl.trim().is_empty() => {
            if let Some(error) = hardening_error {
                return ArlStorageStatus::none(Some(format!(
                    "Cannot safely use the ARL in settings.json: {}",
                    error
                )));
            }
            let reason = if keyring_enabled() {
                load_arl_from_keyring().err()
            } else {
                Some("Disabled by DEEZY_NO_KEYRING".to_string())
            };
            return ArlStorageStatus {
                storage: Some(ArlStorage::PlainFile),
                reason,
            };
        }
        Ok((_, hardening_error)) => disk_hardening_error = hardening_error,
        Err(e) => {
            return ArlStorageStatus::none(Some(format!(
                "Cannot safely read settings.json: {}",
                e
            )));
        }
    }

    if !keyring_enabled() {
        return ArlStorageStatus::none(disk_hardening_error.or_else(|| {
            Some("Disabled by DEEZY_NO_KEYRING".to_string())
        }));
    }

    match load_arl_from_keyring() {
        Ok(Some(arl)) if !arl.trim().is_empty() => ArlStorageStatus {
            storage: Some(ArlStorage::Keyring),
            reason: disk_hardening_error,
        },
        Ok(_) => ArlStorageStatus::none(disk_hardening_error),
        Err(e) => ArlStorageStatus::none(Some(
            disk_hardening_error
                .map(|disk_error| format!("{}; {}", disk_error, e))
                .unwrap_or(e),
        )),
    }
}

fn ensure_regular_file(file: &std::fs::File) -> Result<(), String> {
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("settings.json is not a regular file".to_string());
    }

    Ok(())
}

fn enforce_private_file(file: &std::fs::File) -> Result<(), String> {
    ensure_regular_file(file)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
        let mode = file
            .metadata()
            .map_err(|e| e.to_string())?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err("settings.json permissions are not owner-only".to_string());
        }
    }

    Ok(())
}

#[derive(Debug)]
struct PrivateFileContents {
    data: String,
    hardening_error: Option<String>,
}

fn read_private(path: &Path) -> Result<Option<PrivateFileContents>, String> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK);
    }
    #[cfg(not(unix))]
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err("Refusing to use a symlink for settings.json".to_string());
        }
    }

    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    ensure_regular_file(&file)?;
    let hardening_error = enforce_private_file(&file).err();

    let mut data = String::new();
    (&mut file)
        .take(MAX_SETTINGS_BYTES + 1)
        .read_to_string(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_SETTINGS_BYTES {
        return Err("settings.json is too large".to_string());
    }
    Ok(Some(PrivateFileContents {
        data,
        hardening_error,
    }))
}

fn parse_private_settings(contents: PrivateFileContents) -> Result<Settings, String> {
    let settings: Settings =
        serde_json::from_str(&contents.data).map_err(|e| e.to_string())?;
    if let Some(error) = contents.hardening_error {
        if !settings.arl.trim().is_empty() {
            return Err(format!(
                "Refusing to load a plaintext ARL without private file permissions: {}",
                error
            ));
        }
        eprintln!(
            "Warning: could not restrict permissions on ARL-free settings.json: {}",
            error
        );
    }
    Ok(settings)
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::rename(source, destination).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    match std::fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err("Refusing to replace a non-regular settings.json".to_string());
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }

    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn write_private(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "settings.json has no parent directory".to_string())?;
    let mut last_error = None;

    for _ in 0..10 {
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_path = parent.join(format!(
            ".settings.json.{}.{}.tmp",
            std::process::id(),
            counter
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }

        let mut file = match options.open(&temp_path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(e.to_string());
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };

        let write_result = (|| {
            enforce_private_file(&file)?;
            file.write_all(data).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);

            replace_file(&temp_path, path)?;
            #[cfg(unix)]
            std::fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|e| e.to_string())?;
            Ok(())
        })();

        if write_result.is_err() {
            let _ = std::fs::remove_file(&temp_path);
        }
        return write_result;
    }

    Err(format!(
        "Could not reserve a temporary settings file: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    ))
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

        if self.folder_structure == FolderStructure::Custom && self.custom_folder_template.trim().is_empty() {
            return Err("Custom folder template is required".to_string());
        }

        let serialized = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if serialized.len() as u64 > MAX_SETTINGS_BYTES {
            return Err("Settings are too large to save".to_string());
        }

        Ok(())
    }

    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let path = Self::path(app)?;
        let mut settings: Self = if let Some(contents) = read_private(&path)? {
            parse_private_settings(contents)?
        } else {
            Self::default()
        };

        if !keyring_enabled() {
            return Ok(settings);
        }

        // A credential already in the keyring is authoritative. This also
        // prevents stale plaintext left by an interrupted save from replacing it.
        if let Ok(Some(arl)) = load_arl_from_keyring() {
            if !arl.is_empty() {
                let disk_had_arl = !settings.arl.is_empty();
                settings.arl = arl;
                if disk_had_arl {
                    let mut clean = settings.clone();
                    clean.arl = String::new();
                    let data = serde_json::to_string_pretty(&clean).map_err(|e| e.to_string())?;
                    write_private(&path, data.as_bytes())?;
                }
                return Ok(settings);
            }
        }

        // Migrate a legacy plaintext credential only when the keyring is empty.
        if !settings.arl.is_empty() && save_arl_to_keyring(&settings.arl).is_ok() {
            let mut clean = settings.clone();
            clean.arl = String::new();
            let data = serde_json::to_string_pretty(&clean).map_err(|e| e.to_string())?;
            write_private(&path, data.as_bytes())?;
        }

        Ok(settings)
    }

    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        // Validate before saving
        self.validate()?;

        let path = Self::path(app)?;
        if keyring_enabled() {
            let previous_arl = load_arl_from_keyring()
                .map_err(|e| format!("Cannot safely update credential: {}", e))?;
            match save_arl_to_keyring(&self.arl) {
                Ok(()) => {
                    let mut settings_for_disk = self.clone();
                    settings_for_disk.arl = String::new();
                    let data = serde_json::to_string_pretty(&settings_for_disk)
                        .map_err(|e| e.to_string())?;
                    if let Err(write_error) = write_private(&path, data.as_bytes()) {
                        let rollback = match previous_arl {
                            Some(ref arl) if !arl.is_empty() => save_arl_to_keyring(arl),
                            _ => delete_arl_from_keyring(),
                        };
                        return Err(match rollback {
                            Ok(()) => write_error,
                            Err(rollback_error) => format!(
                                "{}; credential rollback also failed: {}",
                                write_error, rollback_error
                            ),
                        });
                    }
                }
                Err(e) => {
                    if previous_arl.as_deref().is_some_and(|arl| !arl.is_empty()) {
                        return Err(format!(
                            "Secure credential update failed; the previous credential was preserved: {}",
                            e
                        ));
                    }
                    eprintln!(
                        "Warning: secure credential storage unavailable ({}). Storing ARL in settings.json",
                        e
                    );
                    let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
                    write_private(&path, data.as_bytes())?;
                }
            }
        } else {
            let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
            write_private(&path, data.as_bytes())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_private_settings, read_private, write_private, PrivateFileContents, Settings};
    #[cfg(unix)]
    use std::ffi::CString;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(unix)]
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "deezy-settings-{}-{}-{}",
            name,
            std::process::id(),
            nonce
        ));
        std::fs::create_dir(&path).expect("test directory should be created");
        path
    }

    fn read_data(path: &std::path::Path) -> String {
        let contents = read_private(path)
            .expect("private read should succeed")
            .expect("settings file should exist");
        assert!(contents.hardening_error.is_none());
        contents.data
    }

    #[test]
    fn hardening_failure_only_rejects_settings_with_an_arl() {
        let settings = Settings::default();
        let data = serde_json::to_string(&settings).expect("settings should serialize");
        let loaded = parse_private_settings(PrivateFileContents {
            data,
            hardening_error: Some("permission denied".to_string()),
        });
        assert!(loaded.is_ok());

        let mut settings_with_arl = settings;
        settings_with_arl.arl = "secret".to_string();
        let data = serde_json::to_string(&settings_with_arl).expect("settings should serialize");
        let error = parse_private_settings(PrivateFileContents {
            data,
            hardening_error: Some("permission denied".to_string()),
        })
        .expect_err("plaintext ARL must be rejected");
        assert!(error.contains("plaintext ARL"));
    }

    #[test]
    fn private_file_round_trip_succeeds() {
        let dir = test_dir("permissions");
        let path = dir.join("settings.json");

        write_private(&path, br#"{"arl":"secret"}"#).expect("private write should succeed");

        #[cfg(unix)]
        {
            let mode = std::fs::metadata(&path)
                .expect("settings file should exist")
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0);
        }
        assert_eq!(read_data(&path), r#"{"arl":"secret"}"#);

        write_private(&path, b"replacement").expect("private replacement should succeed");
        assert_eq!(read_data(&path), "replacement");

        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn world_readable_settings_are_hardened_before_use() {
        let dir = test_dir("world-readable");
        let path = dir.join("settings.json");
        std::fs::write(&path, b"settings").expect("settings file should be written");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("test permissions should be applied");

        assert_eq!(read_data(&path), "settings");
        let mode = std::fs::metadata(&path)
            .expect("settings file should exist")
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);

        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn settings_symlink_reads_are_rejected() {
        let dir = test_dir("symlink");
        let target = dir.join("target.json");
        let link = dir.join("settings.json");
        std::fs::write(&target, br#"{"arl":"secret"}"#).expect("target should be written");
        symlink(&target, &link).expect("symlink should be created");

        read_private(&link).expect_err("symlink must not be read");

        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn fifo_is_rejected_without_blocking() {
        let dir = test_dir("fifo");
        let path = dir.join("settings.json");
        let c_path = CString::new(path.as_os_str().as_bytes()).expect("path should not contain NUL");
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);

        read_private(&path).expect_err("FIFO must not be read");

        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn private_write_replaces_symlink_without_following_it() {
        let dir = test_dir("symlink-write");
        let target = dir.join("target.json");
        let link = dir.join("settings.json");
        std::fs::write(&target, b"unchanged").expect("target should be written");
        symlink(&target, &link).expect("symlink should be created");

        write_private(&link, b"replacement").expect("private write should replace the link");

        assert_eq!(std::fs::read(&target).expect("target should remain"), b"unchanged");
        assert_eq!(read_data(&link), "replacement");

        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }
}
