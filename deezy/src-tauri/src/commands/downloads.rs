use super::*;

#[tauri::command]
#[allow(non_snake_case)]
pub async fn download_track(
    trackId: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<DownloadResult, String> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut cancellation_map = state.download_cancellations.lock().await;
        if cancellation_map.contains_key(&trackId) {
            return Err("This track is already downloading".to_string());
        }
        cancellation_map.insert(trackId.clone(), cancel_flag.clone());
    }

    let result = execute_download_track(&trackId, &state, &app, cancel_flag).await;

    // Always remove the registration, including authentication and refresh
    // failures that return before the streaming download begins.
    state.download_cancellations.lock().await.remove(&trackId);

    if let Ok(download) = &result {
        if !download.file_path.is_empty() {
            grant_audio_file(&state, &download.file_path).await?;
        }
    }

    result
}

async fn execute_download_track(
    track_id: &str,
    state: &AppState,
    app: &AppHandle,
    cancel_flag: Arc<AtomicBool>,
) -> Result<DownloadResult, String> {
    if crate::spotify::is_spotify_track(track_id) {
        return execute_spotify_download(track_id, state, app, cancel_flag).await;
    }

    // Get or recreate the client
    let (mut client, output_dir, quality, folder_structure, custom_folder_template, arl, skip_existing, speed_limit_kbps) = {
        let lock = state.client.lock().await;
        let settings = state.settings.lock().await;
        
        let client = if let Some(c) = lock.as_ref() {
            c.clone()
        } else {
            return Err("Not logged in. Please set your ARL token in Settings.".to_string());
        };
        
        (
            client,
            settings.output_dir.clone(),
            settings.quality.clone(),
            settings.folder_structure.clone(),
            settings.custom_folder_template.clone(),
            settings.arl.clone(),
            settings.skip_existing_files,
            settings.download_speed_limit_kbps,
        )
    };
    
    // If token is empty or invalid, try to refresh the client
    if client.token.is_empty() && !arl.is_empty() {
        match DeezerClient::new(&arl).await {
            Ok(new_client) => {
                client = new_client.clone();
                install_client_if_current(state, &arl, new_client).await;
            }
            Err(e) => {
                return Err(format!("Failed to refresh session: {}", e));
            }
        }
    }

    let mut effective_quality = quality.clone();
    if client
        .user
        .as_ref()
        .map(|u| u.is_free_account)
        .unwrap_or(false)
        && quality != "MP3_128"
    {
        effective_quality = "MP3_128".to_string();
    }

    let mut result = download::download_track(
        &client,
        track_id,
        &output_dir,
        &effective_quality,
        &folder_structure,
        &custom_folder_template,
        skip_existing,
        &state.bandwidth,
        speed_limit_kbps,
        app,
        cancel_flag.clone(),
    )
    .await;
    
    // If we get a CSRF error, try to refresh the client and retry once
    if let Err(ref e) = result {
        if !cancel_flag.load(Ordering::Relaxed) && (e.contains("CSRF") || e.contains("token")) {
            match DeezerClient::new(&arl).await {
                Ok(new_client) => {
                    client = new_client.clone();
                    install_client_if_current(state, &arl, new_client).await;
                    let mut retry_quality = quality.clone();
                    if client
                        .user
                        .as_ref()
                        .map(|u| u.is_free_account)
                        .unwrap_or(false)
                        && quality != "MP3_128"
                    {
                        retry_quality = "MP3_128".to_string();
                    }

                    result = download::download_track(
                        &client,
                        track_id,
                        &output_dir,
                        &retry_quality,
                        &folder_structure,
                        &custom_folder_template,
                        skip_existing,
                        &state.bandwidth,
                        speed_limit_kbps,
                        app,
                        cancel_flag.clone(),
                    )
                    .await;
                }
                Err(_) => {
                    return Err(format!("Session expired. Please go to Settings and log in again. Error: {}", e));
                }
            }
        }
    }

    result
}

async fn execute_spotify_download(
    track_id: &str,
    state: &AppState,
    app: &AppHandle,
    cancel_flag: Arc<AtomicBool>,
) -> Result<DownloadResult, String> {
    let (client, output_dir, quality, folder_structure, custom_folder_template, skip_existing, speed_limit_kbps) = {
        let client = state
            .spotify
            .lock()
            .await
            .as_ref()
            .cloned()
            .ok_or("Not connected to Spotify. Open Settings and connect your account.")?;
        let settings = state.settings.lock().await;
        (
            client,
            settings.output_dir.clone(),
            settings.spotify_quality.clone(),
            settings.folder_structure.clone(),
            settings.custom_folder_template.clone(),
            settings.skip_existing_files,
            settings.download_speed_limit_kbps,
        )
    };

    crate::spotify::download::download_track(
        &client,
        track_id,
        &output_dir,
        &quality,
        &folder_structure,
        &custom_folder_template,
        skip_existing,
        &state.bandwidth,
        speed_limit_kbps,
        app,
        cancel_flag,
    )
    .await
}

pub(super) async fn install_client_if_current(state: &AppState, refreshed_arl: &str, client: DeezerClient) {
    let _settings_io = state.settings_io.lock().await;
    let arl_is_current = state.settings.lock().await.arl == refreshed_arl;
    if arl_is_current {
        *state.client.lock().await = Some(client);
    }
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn cancel_download(
    trackId: String,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let cancellation_map = state.download_cancellations.lock().await;
    if let Some(flag) = cancellation_map.get(&trackId) {
        flag.store(true, Ordering::Relaxed);
        Ok(true)
    } else {
        Ok(false)
    }
}
