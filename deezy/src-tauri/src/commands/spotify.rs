use super::*;
use crate::spotify::{self, SpotifyAlbum, SpotifyArtist, SpotifyPlaylist, SpotifyTrack};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Serialises connect, session restore and disconnect, from start to finish,
/// so a logout cannot be undone by a restore in flight and two restores never
/// spend the same (rotating) refresh token. A module static rather than an
/// `AppState` field, since there is only one Spotify session per process.
static AUTH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Set while a browser login waits for its callback, which can take minutes.
static LOGIN_PENDING: AtomicBool = AtomicBool::new(false);

/// Holds `LOGIN_PENDING` for one login and clears it however the login ends.
struct PendingLogin;

impl PendingLogin {
    fn claim() -> Result<Self, String> {
        LOGIN_PENDING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| PendingLogin)
            .map_err(|_| {
                "A Spotify login is already waiting for the browser. Finish it there, \
                 or wait for it to time out after 5 minutes."
                    .to_string()
            })
    }
}

impl Drop for PendingLogin {
    fn drop(&mut self) {
        LOGIN_PENDING.store(false, Ordering::Release);
    }
}

/// Installs `client` and stops a replaced client from writing its refreshed
/// tokens over the new ones.
async fn install_client(state: &AppState, client: Arc<spotify::SpotifyClient>) {
    let previous = state.spotify.lock().await.replace(client);
    if let Some(previous) = previous {
        previous.stop_persisting().await;
    }
}

#[tauri::command]
pub async fn spotify_login(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<SpotifyStatus, String> {
    let _pending = PendingLogin::claim()?;
    let _auth = AUTH_LOCK.lock().await;
    // The startup restore may have connected while this waited for the lock;
    // Connect is only offered while disconnected, so do not open a browser.
    if let Some(client) = state.spotify.lock().await.as_ref().cloned() {
        return Ok(status(&client));
    }
    let device_id = ensure_device_id(&state, &app).await?;
    let client_id = configured_client_id(&state).await;
    let client = spotify::auth::login(&device_id, &client_id).await.map_err(|e| {
        crate::logging::warn("spotify", format!("Connect failed: {}", e));
        e
    })?;
    crate::logging::info(
        "spotify",
        format!("Connected ({}, {} Client ID)", client.profile.product, client_id_kind(&client_id)),
    );
    let status = status(&client);
    install_client(&state, client).await;
    Ok(status)
}

#[tauri::command]
pub async fn spotify_logout(state: tauri::State<'_, AppState>) -> Result<SpotifyStatus, String> {
    let _auth = AUTH_LOCK.lock().await;
    let previous = state.spotify.lock().await.take();
    if let Some(previous) = previous {
        // A refresh still running on this client must not re-save its token.
        previous.stop_persisting().await;
    }
    crate::logging::info("spotify", "Disconnected");
    spotify::auth::logout().await?;
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
    // Do not wait minutes behind a browser login; it reports its own result.
    if LOGIN_PENDING.load(Ordering::Acquire) {
        return Ok(disconnected());
    }

    let _auth = AUTH_LOCK.lock().await;
    // A login or another restore may have finished while this one waited.
    if let Some(client) = state.spotify.lock().await.as_ref().cloned() {
        return Ok(status(&client));
    }

    let device_id = ensure_device_id(&state, &app).await?;
    let client_id = configured_client_id(&state).await;
    let restored = spotify::auth::restore(&device_id, &client_id).await.map_err(|e| {
        crate::logging::warn("spotify", format!("Restoring the session failed: {}", e));
        e
    })?;
    match restored {
        Some(client) => {
            crate::logging::info(
                "spotify",
                format!("Session restored ({}, {} Client ID)", client.profile.product, client_id_kind(&client_id)),
            );
            let status = status(&client);
            install_client(&state, client).await;
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
    // Accepts both a bare ID and a `spotify:<id>` track key.
    let id = spotify::validate_id(spotify::bare_track_id(&trackId), "track")?;
    let client = spotify_client(&state).await?;
    spotify::ap::track(&client.session, id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_spotify_album_tracks(
    albumId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyTrack>, String> {
    let id = spotify::validate_id(&albumId, "album")?;
    let client = spotify_client(&state).await?;
    spotify::ap::album_tracks(&client.session, id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_spotify_artist_albums(
    artistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyAlbum>, String> {
    let id = spotify::validate_id(&artistId, "artist")?;
    let client = spotify_client(&state).await?;
    client.artist_albums(id).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_spotify_playlist_tracks(
    playlistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpotifyTrack>, String> {
    let id = spotify::validate_id(&playlistId, "playlist")?;
    let client = spotify_client(&state).await?;
    spotify::ap::playlist_tracks(&client.session, id).await
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

async fn configured_client_id(state: &AppState) -> String {
    let saved = state.settings.lock().await.spotify_client_id.clone();
    spotify::auth::configured_client_id(&saved)
}

async fn ensure_device_id(state: &AppState, app: &AppHandle) -> Result<String, String> {
    let existing = state.settings.lock().await.spotify_device_id.clone();
    if !existing.is_empty() {
        return Ok(existing);
    }

    let _settings_io = state.settings_io.lock().await;
    // Another caller may have generated and saved one while this waited.
    let mut updated = state.settings.lock().await.clone();
    if !updated.spotify_device_id.is_empty() {
        return Ok(updated.spotify_device_id);
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

    updated.spotify_device_id = generated.clone();
    // Save first (on a blocking thread), and only then keep the ID in memory,
    // so a failed save never leaves an unsaved ID behind.
    let app = app.clone();
    run_blocking(move || updated.save(&app)).await?;
    state.settings.lock().await.spotify_device_id = generated.clone();
    Ok(generated)
}

/// Whether the user's own Client ID is in use, for the log (never the ID).
fn client_id_kind(client_id: &str) -> &'static str {
    if spotify::auth::is_default_client_id(client_id) {
        "default"
    } else {
        "custom"
    }
}

#[cfg(test)]
mod tests {
    use super::PendingLogin;

    #[test]
    fn only_one_browser_login_waits_at_a_time() {
        let first = PendingLogin::claim().expect("first login claims the slot");
        assert!(PendingLogin::claim().is_err());
        drop(first);
        let again = PendingLogin::claim().expect("slot is free once the login ends");
        drop(again);
    }
}
