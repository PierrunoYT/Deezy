use super::*;

#[tauri::command]
pub async fn search_tracks(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    client.search_tracks(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_track_by_id(
    trackId: String,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResult, String> {
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    client.get_track_by_id(&trackId).await
}

#[tauri::command]
pub async fn search_albums(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AlbumResult>, String> {
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    client.search_albums(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_album_tracks(
    albumId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    client.get_album_tracks(&albumId).await
}

#[tauri::command]
pub async fn search_artists(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ArtistResult>, String> {
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    client.search_artists(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_artist_albums(
    artistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AlbumResult>, String> {
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    client.get_artist_albums(&artistId).await
}

#[tauri::command]
pub async fn search_playlists(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PlaylistResult>, String> {
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    client.search_playlists(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_playlist_tracks(
    playlistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    // Checked here too, so a malformed ID doesn't fall through to the private API.
    crate::deezer::deezer_id(&playlistId)?;
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    match client.get_playlist_tracks(&playlistId).await {
        Ok(tracks) => Ok(tracks),
        // The public API refuses private playlists, such as the user's own.
        Err(public_error) => with_session_retry(&state, |client| {
            let playlist_id = playlistId.clone();
            async move { client.get_private_playlist_tracks(&playlist_id).await }
        })
        .await
        .map_err(|_| public_error),
    }
}

#[tauri::command]
pub async fn get_loved_tracks(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    with_session_retry(&state, |client| async move { client.get_loved_tracks().await }).await
}

#[tauri::command]
pub async fn get_user_playlists(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PlaylistResult>, String> {
    with_session_retry(&state, |client| async move { client.get_user_playlists().await }).await
}

/// Runs a private API call, re-authenticating and retrying once when the
/// session's CSRF token has expired.
async fn with_session_retry<T, F, Fut>(state: &AppState, call: F) -> Result<T, String>
where
    F: Fn(DeezerClient) -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let client = {
        let lock = state.client.lock().await;
        lock
        .as_ref()
        .cloned()
        .ok_or("Not logged in. Set your ARL token in Settings.")?
    };
    match call(client).await {
        Err(e) if e.contains("CSRF") || e.contains("token") => {
            let arl = state.settings.lock().await.arl.clone();
            let client = DeezerClient::new(&arl).await.map_err(|_| {
                format!("Session expired. Please go to Settings and log in again. Error: {}", e)
            })?;
            super::downloads::install_client_if_current(state, &arl, client.clone()).await;
            call(client).await
        }
        result => result,
    }
}

