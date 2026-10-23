use crate::deezer::download;
use crate::deezer::models::{AlbumResult, ArtistResult, DownloadResult, FileTagData, PlaylistResult, SearchResult, WriteTagData};
use crate::deezer::DeezerClient;
use crate::settings::{ArlStorageStatus, Settings};
use crate::themes as theme_store;
use crate::tray as app_tray;
use crate::AppState;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use url::Url;

mod account;
mod catalog;
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
    let canonical = canonical_file(path).await?;
    if state.file_grants.lock().await.audio.contains(&canonical) {
        return Ok(canonical);
    }

    let output_dir = state.settings.lock().await.output_dir.clone();
    if let Ok(output_dir) = canonical_folder(&output_dir).await {
        if canonical.starts_with(output_dir) {
            return Ok(canonical);
        }
    }

    Err("Audio file has not been approved by Deezy".to_string())
}

async fn grant_image_file(state: &AppState, path: &str) -> Result<(), String> {
    let canonical = canonical_file(path).await?;
    state.file_grants.lock().await.images.insert(canonical);
    Ok(())
}

async fn require_image_file(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let canonical = canonical_file(path).await?;
    if !state.file_grants.lock().await.images.contains(&canonical) {
        return Err("Cover image has not been approved by Deezy".to_string());
    }
    Ok(canonical)
}

async fn grant_folder(state: &AppState, path: &str) -> Result<(), String> {
    let canonical = canonical_folder(path).await?;
    state.file_grants.lock().await.folders.insert(canonical);
    Ok(())
}

async fn require_granted_folder(state: &AppState, path: &str) -> Result<(), String> {
    let canonical = canonical_folder(path).await?;
    if !state.file_grants.lock().await.folders.contains(&canonical) {
        return Err("Download folder has not been approved by Deezy".to_string());
    }
    Ok(())
}
