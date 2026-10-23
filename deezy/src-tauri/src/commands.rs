use crate::deezer::download;
use crate::deezer::models::{AlbumResult, ArtistResult, DownloadResult, FileTagData, PlaylistResult, SearchResult, WriteTagData};
use crate::deezer::DeezerClient;
use crate::settings::{ArlStorageStatus, Settings};
use crate::themes as theme_store;
use crate::tray as app_tray;
use crate::{AppState, FileGrants};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use url::Url;

mod account;
mod catalog;
mod diagnostics;
mod downloads;
mod filesystem;
mod history;
mod settings;
mod spotify;
mod tag_editor;
mod themes;
mod tray;

pub use account::*;
pub use catalog::*;
pub use diagnostics::*;
pub use downloads::*;
pub use filesystem::*;
pub use history::*;
pub use settings::*;
pub use spotify::*;
pub use tag_editor::*;
pub use themes::*;
pub use tray::*;

async fn run_blocking<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
}

/// Starts a helper program (a file manager) and reaps it in the background,
/// so it doesn't linger as a zombie process on Unix once it exits.
fn spawn_and_reap(command: &mut Command) -> std::io::Result<()> {
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

// The answer for a renderer path that may not be used. Each is the same
// whether the path is missing, a folder, unreachable or simply not granted, so
// a failure can't be used to probe the disk or the network.
const AUDIO_NOT_APPROVED: &str = "Audio file is missing or has not been approved by Wavekeep";
const IMAGE_NOT_APPROVED: &str = "Cover image is missing or has not been approved by Wavekeep";
const FOLDER_NOT_APPROVED: &str = "Download folder is missing or has not been approved by Wavekeep";

/// A Windows network share as `(server, share)`, lower-cased.
type NetworkShare = (String, String);

/// The share of a Windows network path such as `\\nas\music\a.mp3` or its
/// verbatim form `\\?\UNC\nas\music\a.mp3`, which is what `canonicalize`
/// returns for it.
#[cfg(windows)]
fn network_share(path: &Path) -> Option<NetworkShare> {
    use std::path::{Component, Prefix};
    match path.components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => Some((
                server.to_string_lossy().to_lowercase(),
                share.to_string_lossy().to_lowercase(),
            )),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(not(windows))]
fn network_share(_path: &Path) -> Option<NetworkShare> {
    None
}

/// Decides from the text alone, before any filesystem call, whether a path
/// from the renderer may be resolved. On Windows, resolving a UNC path
/// (`\\host\share\...`) opens an SMB session that sends the user's NTLM
/// credentials to that host, and device paths (`\\.\`, `\\?\GLOBALROOT`) reach
/// past the file system. Only absolute local drive paths pass, plus network
/// shares the user already chose (a picked file or folder, the download folder).
fn may_resolve(path: &Path, trusted_shares: &[NetworkShare]) -> bool {
    if !path.is_absolute() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(_) | Prefix::VerbatimDisk(_) => true,
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) => {
                    network_share(path).is_some_and(|share| trusted_shares.contains(&share))
                }
                _ => false,
            },
            _ => false,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = trusted_shares;
        true
    }
}

/// The network shares of everything already granted and of the download folder.
fn trusted_shares(grants: &FileGrants, output_dir: &str) -> Vec<NetworkShare> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let mut shares: Vec<NetworkShare> = grants
        .audio
        .iter()
        .chain(&grants.images)
        .chain(&grants.folders)
        .map(PathBuf::as_path)
        .chain(std::iter::once(Path::new(output_dir)))
        .filter_map(network_share)
        .collect();
    shares.sort();
    shares.dedup();
    shares
}

/// Resolves a renderer path, or `None` when it fails [`may_resolve`], is
/// missing or unreadable, or is not of the wanted kind.
async fn resolve_requested(path: &str, want_folder: bool, trusted_shares: &[NetworkShare]) -> Option<PathBuf> {
    if !may_resolve(Path::new(path), trusted_shares) {
        return None;
    }
    let canonical = tokio::fs::canonicalize(path).await.ok()?;
    let metadata = tokio::fs::metadata(&canonical).await.ok()?;
    let right_kind = if want_folder { metadata.is_dir() } else { metadata.is_file() };
    right_kind.then_some(canonical)
}

/// Canonicalises a path Wavekeep itself chose (a dialog result, the saved
/// download folder, a finished download). Renderer paths go through
/// [`resolve_requested`] instead.
async fn canonical_file(path: &str) -> Result<PathBuf, String> {
    let canonical = tokio::fs::canonicalize(path)
        .await
        .map_err(|e| format!("Failed to resolve file path: {}", e))?;
    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to inspect file: {}", e))?;
    if !metadata.is_file() {
        return Err("Path is not a file".to_string());
    }
    Ok(canonical)
}

async fn canonical_folder(path: &str) -> Result<PathBuf, String> {
    let canonical = tokio::fs::canonicalize(path)
        .await
        .map_err(|e| format!("Failed to resolve folder path: {}", e))?;
    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to inspect folder: {}", e))?;
    if !metadata.is_dir() {
        return Err("Path is not a folder".to_string());
    }
    Ok(canonical)
}

async fn grant_audio_file(state: &AppState, path: &str) -> Result<(), String> {
    let canonical = canonical_file(path).await?;
    state.file_grants.lock().await.audio.insert(canonical);
    Ok(())
}

async fn require_audio_file(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let output_dir = state.settings.lock().await.output_dir.clone();
    check_audio_file(&state.file_grants, &output_dir, path).await
}

/// A granted audio file, or any file inside the download folder.
async fn check_audio_file(
    grants: &tokio::sync::Mutex<FileGrants>,
    output_dir: &str,
    path: &str,
) -> Result<PathBuf, String> {
    let shares = trusted_shares(&*grants.lock().await, output_dir);
    let canonical = resolve_requested(path, false, &shares)
        .await
        .ok_or_else(|| AUDIO_NOT_APPROVED.to_string())?;
    if grants.lock().await.audio.contains(&canonical) {
        return Ok(canonical);
    }

    if let Ok(output_dir) = canonical_folder(output_dir).await {
        if canonical.starts_with(output_dir) {
            return Ok(canonical);
        }
    }

    Err(AUDIO_NOT_APPROVED.to_string())
}

async fn grant_image_file(state: &AppState, path: &str) -> Result<(), String> {
    let canonical = canonical_file(path).await?;
    state.file_grants.lock().await.images.insert(canonical);
    Ok(())
}

async fn require_image_file(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let output_dir = state.settings.lock().await.output_dir.clone();
    check_image_file(&state.file_grants, &output_dir, path).await
}

/// A cover image picked through `pick_cover_image`.
async fn check_image_file(
    grants: &tokio::sync::Mutex<FileGrants>,
    output_dir: &str,
    path: &str,
) -> Result<PathBuf, String> {
    let shares = trusted_shares(&*grants.lock().await, output_dir);
    match resolve_requested(path, false, &shares).await {
        Some(canonical) if grants.lock().await.images.contains(&canonical) => Ok(canonical),
        _ => Err(IMAGE_NOT_APPROVED.to_string()),
    }
}

async fn grant_folder(state: &AppState, path: &str) -> Result<(), String> {
    let canonical = canonical_folder(path).await?;
    state.file_grants.lock().await.folders.insert(canonical);
    Ok(())
}

async fn require_granted_folder(state: &AppState, path: &str) -> Result<(), String> {
    let output_dir = state.settings.lock().await.output_dir.clone();
    check_granted_folder(&state.file_grants, &output_dir, path).await
}

/// A folder picked through `pick_folder` (or the saved download folder).
async fn check_granted_folder(
    grants: &tokio::sync::Mutex<FileGrants>,
    output_dir: &str,
    path: &str,
) -> Result<(), String> {
    let shares = trusted_shares(&*grants.lock().await, output_dir);
    match resolve_requested(path, true, &shares).await {
        Some(canonical) if grants.lock().await.folders.contains(&canonical) => Ok(()),
        _ => Err(FOLDER_NOT_APPROVED.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;

    fn test_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wavekeep-grants-{}-{}-{}", name, std::process::id(), nonce));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn relative_paths_are_never_resolved() {
        for path in ["a.mp3", "music/a.mp3", "", "."] {
            assert!(!may_resolve(Path::new(path), &[]), "{}", path);
        }
    }

    #[cfg(windows)]
    #[test]
    fn only_local_drives_and_chosen_shares_are_resolved() {
        for local in [r"C:\Music\a.mp3", r"\\?\C:\Music\a.mp3", "D:/Music/a.mp3"] {
            assert!(may_resolve(Path::new(local), &[]), "{}", local);
        }
        for rejected in [
            r"\\attacker.example\s\x.mp3",
            r"\\127.0.0.1\C$\Windows\win.ini",
            r"\\?\UNC\attacker.example\s\x.mp3",
            r"\\.\UNC\attacker.example\s\x.mp3",
            r"\\.\COM1",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1\a.mp3",
            r"\\nas@SSL\music\a.mp3",
            r"C:Music\a.mp3",
            r"\Music\a.mp3",
            "//attacker.example/s/x.mp3",
        ] {
            assert!(!may_resolve(Path::new(rejected), &[]), "{}", rejected);
        }

        let mut grants = FileGrants::default();
        grants.folders.insert(PathBuf::from(r"\\?\UNC\NAS\Music\Wavekeep"));
        let shares = trusted_shares(&grants, r"C:\Music");
        assert_eq!(shares, vec![("nas".to_string(), "music".to_string())]);
        for chosen in [r"\\nas\music\a.mp3", r"\\NAS\Music\Album\a.flac", r"\\?\UNC\nas\music\a.mp3"] {
            assert!(may_resolve(Path::new(chosen), &shares), "{}", chosen);
        }
        for other in [r"\\nas\private\a.mp3", r"\\nas.evil.example\music\a.mp3"] {
            assert!(!may_resolve(Path::new(other), &shares), "{}", other);
        }
        // A download folder on a share is trusted even before it is granted.
        assert_eq!(trusted_shares(&FileGrants::default(), r"\\nas\music\Wavekeep"), shares);
    }

    #[tokio::test]
    async fn every_refusal_reads_the_same_whatever_the_path_state() {
        let dir = test_dir("uniform");
        let output_dir = dir.join("downloads");
        std::fs::create_dir_all(&output_dir).unwrap();
        let outside = dir.join("outside.mp3");
        std::fs::write(&outside, b"audio").unwrap();
        let inside = output_dir.join("inside.mp3");
        std::fs::write(&inside, b"audio").unwrap();
        let output = output_dir.to_string_lossy().into_owned();
        let grants = Mutex::new(FileGrants::default());

        let mut refused = vec![
            outside.to_string_lossy().into_owned(),
            dir.join("missing.mp3").to_string_lossy().into_owned(),
            dir.to_string_lossy().into_owned(),
            "relative.mp3".to_string(),
        ];
        if cfg!(windows) {
            refused.push(r"\\attacker.example\s\x.mp3".to_string());
            refused.push(r"\\.\COM1".to_string());
        }
        for path in &refused {
            assert_eq!(check_audio_file(&grants, &output, path).await, Err(AUDIO_NOT_APPROVED.to_string()), "{}", path);
            assert_eq!(check_image_file(&grants, &output, path).await, Err(IMAGE_NOT_APPROVED.to_string()), "{}", path);
            assert_eq!(check_granted_folder(&grants, &output, path).await, Err(FOLDER_NOT_APPROVED.to_string()), "{}", path);
        }

        // Files inside the download folder and granted files still pass.
        assert!(check_audio_file(&grants, &output, &inside.to_string_lossy()).await.is_ok());
        let canonical = std::fs::canonicalize(&outside).unwrap();
        grants.lock().await.audio.insert(canonical.clone());
        grants.lock().await.images.insert(canonical.clone());
        assert_eq!(check_audio_file(&grants, &output, &outside.to_string_lossy()).await, Ok(canonical.clone()));
        assert_eq!(check_image_file(&grants, &output, &outside.to_string_lossy()).await, Ok(canonical));
        grants.lock().await.folders.insert(std::fs::canonicalize(&output_dir).unwrap());
        assert!(check_granted_folder(&grants, &output, &output).await.is_ok());

        let _ = std::fs::remove_dir_all(dir);
    }
}
