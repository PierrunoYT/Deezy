//! One-time move of Deezy's data and saved logins to Wavekeep.
//!
//! Up to 0.3.0 the app identifier was `com.pierr.deezy`. The identifier names
//! both the app-data folder and the OS credential-store service, so a new one
//! starts with neither. On startup this moves settings, download history and
//! custom themes into the new folder and moves the ARL and Spotify credentials
//! to the new service. Each item is skipped when the new location already has
//! it, so running the migration again does nothing.

use std::fs;
use std::io;
use std::path::Path;

use tauri::Manager;

use crate::spotify::debug_log;

/// The app identifier. Must match `identifier` in `tauri.conf.json`; it is
/// also the credential-store service name.
pub const IDENTIFIER: &str = "com.pierr.wavekeep";
const OLD_IDENTIFIER: &str = "com.pierr.deezy";

/// Everything the app keeps in its app-data folder.
const DATA_ENTRIES: [&str; 3] = ["settings.json", "download_history.json", "themes"];

/// Runs before any command can read settings or credentials: the setup hook
/// finishes before the event loop serves the first IPC call.
pub fn run(app: &tauri::AppHandle) {
    let Ok(new_dir) = app.path().app_data_dir() else {
        return;
    };
    // Only ever touch the sibling folder of our own data folder.
    if new_dir.file_name().and_then(|name| name.to_str()) != Some(IDENTIFIER) {
        debug_log("migration: unexpected app-data folder, skipping");
        return;
    }
    let Some(old_dir) = new_dir.parent().map(|parent| parent.join(OLD_IDENTIFIER)) else {
        return;
    };

    for line in migrate_data(&old_dir, &new_dir) {
        debug_log(&format!("migration: {}", line));
    }

    if crate::settings::keyring_enabled() {
        let users = [
            crate::settings::KEYRING_USER,
            crate::spotify::auth::KEYRING_USER,
            crate::spotify::auth::KEYRING_WEB_USER,
        ];
        for line in migrate_credentials(&Keyring, OLD_IDENTIFIER, IDENTIFIER, &users) {
            debug_log(&format!("migration: {}", line));
        }
    }
}

/// Moves the app-data entries from `old` to `new` and returns what happened.
fn migrate_data(old: &Path, new: &Path) -> Vec<String> {
    let mut log = Vec::new();
    if !old.is_dir() {
        return log;
    }
    if let Err(e) = fs::create_dir_all(new) {
        log.push(format!("cannot create {}: {}", new.display(), e));
        return log;
    }

    for name in DATA_ENTRIES {
        let (from, to) = (old.join(name), new.join(name));
        if !from.exists() {
            continue;
        }
        if to.exists() {
            log.push(format!("kept the existing {}; the Deezy copy stays in place", name));
            continue;
        }
        match move_entry(&from, &to) {
            Ok(()) => log.push(format!("moved {}", name)),
            Err(e) => log.push(format!("could not move {}: {}", name, e)),
        }
    }

    // Only an empty folder is removed, so nothing unknown is ever deleted.
    if fs::remove_dir(old).is_ok() {
        log.push("removed the empty Deezy data folder".to_string());
    }
    log
}

fn move_entry(from: &Path, to: &Path) -> io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // A rename can fail across volumes or on a locked file: copy instead, and
    // only remove the original once the copy is complete.
    let copied = if from.is_dir() {
        copy_dir(from, to)
    } else {
        fs::copy(from, to).map(|_| ())
    };
    if let Err(e) = copied {
        let _ = if to.is_dir() { fs::remove_dir_all(to) } else { fs::remove_file(to) };
        return Err(e);
    }
    let _ = if from.is_dir() { fs::remove_dir_all(from) } else { fs::remove_file(from) };
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The credential store, abstracted so the migration can be tested without
/// touching the real OS keyring.
trait CredentialStore {
    fn get(&self, service: &str, user: &str) -> Result<Option<String>, String>;
    fn set(&self, service: &str, user: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, service: &str, user: &str) -> Result<(), String>;
}

struct Keyring;

impl CredentialStore for Keyring {
    fn get(&self, service: &str, user: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(service, user).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, service: &str, user: &str, secret: &str) -> Result<(), String> {
        keyring::Entry::new(service, user)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|e| e.to_string())
    }

    fn delete(&self, service: &str, user: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(service, user).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Moves each credential from the `old` service to `new`. The old entry is
/// deleted only after the new one reads back correctly (or when the new
/// service already holds a newer credential), so a secret is never lost and
/// never left behind under the old name.
fn migrate_credentials(
    store: &impl CredentialStore,
    old: &str,
    new: &str,
    users: &[&str],
) -> Vec<String> {
    let mut log = Vec::new();
    for user in users {
        let secret = match store.get(old, user) {
            Ok(Some(secret)) => secret,
            Ok(None) => continue,
            Err(e) => {
                log.push(format!("could not read the old {}: {}", user, e));
                continue;
            }
        };

        match store.get(new, user) {
            Ok(Some(_)) => log.push(format!("kept the newer {}", user)),
            Ok(None) => {
                if let Err(e) = store.set(new, user, &secret) {
                    log.push(format!("could not save {}: {}; kept the old entry", user, e));
                    continue;
                }
                if store.get(new, user).ok().flatten().as_deref() != Some(secret.as_str()) {
                    log.push(format!("{} did not read back; kept the old entry", user));
                    continue;
                }
                log.push(format!("moved {}", user));
            }
            Err(e) => {
                log.push(format!("could not check the new {}: {}", user, e));
                continue;
            }
        }

        if let Err(e) = store.delete(old, user) {
            log.push(format!("could not remove the old {}: {}", user, e));
        }
    }
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "wavekeep-migration-{}-{}-{}",
            name,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn identifier_matches_the_tauri_config() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"], IDENTIFIER);
    }

    #[test]
    fn moves_settings_history_and_themes_and_removes_the_empty_folder() {
        let root = temp_root("move");
        let (old, new) = (root.join(OLD_IDENTIFIER), root.join(IDENTIFIER));
        write(&old.join("settings.json"), "{\"theme\":\"dark\"}");
        write(&old.join("download_history.json"), "[]");
        write(&old.join("themes").join("mine.json"), "{}");

        let log = migrate_data(&old, &new);

        assert_eq!(fs::read_to_string(new.join("settings.json")).unwrap(), "{\"theme\":\"dark\"}");
        assert!(new.join("download_history.json").is_file());
        assert!(new.join("themes").join("mine.json").is_file());
        assert!(!old.exists(), "{:?}", log);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn never_overwrites_new_data_or_deletes_unknown_files() {
        let root = temp_root("keep");
        let (old, new) = (root.join(OLD_IDENTIFIER), root.join(IDENTIFIER));
        write(&old.join("settings.json"), "old");
        write(&old.join("something-else.txt"), "x");
        write(&new.join("settings.json"), "new");

        migrate_data(&old, &new);

        assert_eq!(fs::read_to_string(new.join("settings.json")).unwrap(), "new");
        assert_eq!(fs::read_to_string(old.join("settings.json")).unwrap(), "old");
        assert!(old.join("something-else.txt").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn does_nothing_without_a_deezy_folder_and_is_repeatable() {
        let root = temp_root("noop");
        let (old, new) = (root.join(OLD_IDENTIFIER), root.join(IDENTIFIER));
        assert!(migrate_data(&old, &new).is_empty());
        assert!(!new.exists());

        write(&old.join("settings.json"), "{}");
        migrate_data(&old, &new);
        assert!(migrate_data(&old, &new).is_empty());
        assert!(new.join("settings.json").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn copies_nested_theme_folders() {
        let root = temp_root("copy");
        write(&root.join("a").join("b").join("c.json"), "1");
        copy_dir(&root.join("a"), &root.join("z")).unwrap();
        assert_eq!(fs::read_to_string(root.join("z").join("b").join("c.json")).unwrap(), "1");
        let _ = fs::remove_dir_all(root);
    }

    #[derive(Default)]
    struct MemoryStore {
        entries: RefCell<HashMap<(String, String), String>>,
        fail_set: bool,
    }

    impl MemoryStore {
        fn with(entries: &[(&str, &str, &str)]) -> Self {
            let store = Self::default();
            for (service, user, secret) in entries {
                store.set(service, user, secret).unwrap();
            }
            store
        }

        fn value(&self, service: &str, user: &str) -> Option<String> {
            self.get(service, user).unwrap()
        }
    }

    impl CredentialStore for MemoryStore {
        fn get(&self, service: &str, user: &str) -> Result<Option<String>, String> {
            Ok(self.entries.borrow().get(&(service.into(), user.into())).cloned())
        }

        fn set(&self, service: &str, user: &str, secret: &str) -> Result<(), String> {
            if self.fail_set {
                return Err("locked".into());
            }
            self.entries
                .borrow_mut()
                .insert((service.into(), user.into()), secret.into());
            Ok(())
        }

        fn delete(&self, service: &str, user: &str) -> Result<(), String> {
            self.entries.borrow_mut().remove(&(service.into(), user.into()));
            Ok(())
        }
    }

    #[test]
    fn moves_credentials_and_removes_the_old_copies() {
        let store = MemoryStore::with(&[("old", "arl", "secret-arl"), ("old", "spotify", "token")]);
        migrate_credentials(&store, "old", "new", &["arl", "spotify", "missing"]);

        assert_eq!(store.value("new", "arl").as_deref(), Some("secret-arl"));
        assert_eq!(store.value("new", "spotify").as_deref(), Some("token"));
        assert_eq!(store.value("old", "arl"), None);
        assert_eq!(store.value("old", "spotify"), None);
        assert_eq!(store.value("new", "missing"), None);
    }

    #[test]
    fn keeps_a_newer_credential_and_drops_the_stale_one() {
        let store = MemoryStore::with(&[("old", "arl", "stale"), ("new", "arl", "fresh")]);
        migrate_credentials(&store, "old", "new", &["arl"]);

        assert_eq!(store.value("new", "arl").as_deref(), Some("fresh"));
        assert_eq!(store.value("old", "arl"), None);
    }

    #[test]
    fn keeps_the_old_credential_when_saving_fails() {
        let mut store = MemoryStore::with(&[("old", "arl", "secret")]);
        store.fail_set = true;
        let log = migrate_credentials(&store, "old", "new", &["arl"]);

        assert_eq!(store.value("old", "arl").as_deref(), Some("secret"));
        assert_eq!(store.value("new", "arl"), None);
        assert!(log[0].contains("kept the old entry"));
    }
}
