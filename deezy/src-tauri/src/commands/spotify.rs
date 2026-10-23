use super::*;
use crate::spotify::{self, SpotifyAlbum, SpotifyArtist, SpotifyPlaylist, SpotifyTrack};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct SpotifyStatus {
    pub connected: bool,
    pub name: Option<String>,
    pub product: Option<String>,
    pub image: Option<String>,
}

fn status(client: &spotify::SpotifyClient) -> SpotifyStatus {
    SpotifyStatus {
        connected: true,
        name: Some(client.profile.name.clone()),
        product: Some(client.profile.product.clone()),
        image: client.profile.image.clone(),
    }
}

fn disconnected() -> SpotifyStatus {
    SpotifyStatus {
        connected: false,
        name: None,
        product: None,
        image: None,
    }
}

#[tauri::command]
pub async fn spotify_login(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<SpotifyStatus, String> {
    let device_id = ensure_device_id(&state, &app).await?;
    let client_id = spotify::auth::configured_client_id();
    let client = spotify::auth::login(&device_id, &client_id).await?;
    let status = status(&client);
    *state.spotify.lock().await = Some(client);
    Ok(status)
}

#[tauri::command]
pub async fn spotify_logout(state: tauri::State<'_, AppState>) -> Result<SpotifyStatus, String> {
    *state.spotify.lock().await = None;
    spotify::auth::logout()?;
    Ok(disconnected())
}

#[tauri::command]
pub async fn spotify_status(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<SpotifyStatus, String> {
    if let Some(client) = state.spotify.lock().await.as_ref().cloned() {
        return Ok(status(&client));
    }

    let device_id = ensure_device_id(&state, &app).await?;
    let client_id = spotify::auth::configured_client_id();
    match spotify::auth::restore(&device_id, &client_id).await? {
        Some(client) => {
            let status = status(&client);
            *state.spotify.lock().await = Some(client);
            Ok(status)
        }
        None => Ok(disconnected()),
    }
}

#[tauri::command]
pub async fn search_spotify_tracks(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyTrack>, String> {
    spotify_client(&state).await?.search_tracks(&query, 20).await
}

#[tauri::command]
pub async fn search_spotify_albums(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyAlbum>, String> {
    spotify_client(&state).await?.search_albums(&query, 20).await
}

#[tauri::command]
pub async fn search_spotify_artists(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyArtist>, String> {
    spotify_client(&state).await?.search_artists(&query, 20).await
}

#[tauri::command]
pub async fn search_spotify_playlists(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyPlaylist>, String> {
    spotify_client(&state).await?.search_playlists(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_spotify_track(
    trackId: String,
    state: tauri::State<'_, AppState>,
) -> Result<SpotifyTrack, String> {
    let client = spotify_client(&state).await?;
    spotify::ap::track(&client.session, spotify::bare_track_id(&trackId)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_spotify_album_tracks(
    albumId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyTrack>, String> {
    let client = spotify_client(&state).await?;
    spotify::ap::album_tracks(&client.session, &albumId).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_spotify_artist_albums(
    artistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyAlbum>, String> {
    let client = spotify_client(&state).await?;
    spotify::ap::artist_albums(&client.session, &artistId).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_spotify_playlist_tracks(
    playlistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyTrack>, String> {
    let client = spotify_client(&state).await?;
    spotify::ap::playlist_tracks(&client.session, &playlistId).await
}

#[tauri::command]
pub async fn get_spotify_loved_tracks(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyTrack>, String> {
    let client = spotify_client(&state).await?;
    spotify::ap::liked_tracks(&client.session, &client.profile.id).await
}

#[tauri::command]
pub async fn get_spotify_user_playlists(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyPlaylist>, String> {
    spotify_client(&state).await?.user_playlists().await
}

async fn spotify_client(state: &AppState) -> Result<Arc<spotify::SpotifyClient>, String> {
    if let Some(client) = state.spotify.lock().await.as_ref().cloned() {
        return Ok(client);
    }
    Err("Not connected to Spotify. Open Settings and connect your account.".to_string())
}

async fn ensure_device_id(state: &AppState, app: &AppHandle) -> Result<String, String> {
    let existing = state.settings.lock().await.spotify_device_id.clone();
    if !existing.is_empty() {
        return Ok(existing);
    }

    let generated = {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        format!(
            "{:x}",
            md5::compute(format!("{}-{}", std::process::id(), nanos))
        )
    };

    let _settings_io = state.settings_io.lock().await;
    let snapshot = {
        let mut settings = state.settings.lock().await;
        settings.spotify_device_id = generated.clone();
        settings.clone()
    };
    snapshot.save(app)?;
    Ok(generated)
}
