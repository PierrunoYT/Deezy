use super::*;

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Settings, String> {
    let _settings_io = state.settings_io.lock().await;
    let loaded = run_blocking(move || Settings::load(&app)).await?;
    {
        let mut settings = state.settings.lock().await;
        *settings = loaded.clone();
    }
    let _ = grant_folder(&state, &loaded.output_dir).await;

    // Never return ARL to the renderer process.
    let mut safe = loaded;
    safe.arl = String::new();
    Ok(safe)
}

#[tauri::command]
/// Returns the persisted ARL storage location while serializing access with settings writes.
pub async fn get_arl_storage_status(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<ArlStorageStatus, String> {
    let _settings_io = state.settings_io.lock().await;
    run_blocking(move || Ok(crate::settings::arl_storage_status(&app))).await
}

#[tauri::command]
pub async fn save_settings(
    new_settings: Settings,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let current_output_dir = state.settings.lock().await.output_dir.clone();
    if new_settings.output_dir != current_output_dir {
        require_granted_folder(&state, &new_settings.output_dir).await?;
    }

    let _settings_io = state.settings_io.lock().await;
    let settings = state.settings.lock().await;
    let mut merged = new_settings.clone();

    // Allow non-auth settings updates without exposing ARL to the renderer.
    if merged.arl.trim().is_empty() {
        merged.arl = settings.arl.clone();
    }
    // Search history is backend-owned and must not be erased by a stale or
    // intentionally redacted full-settings payload from the renderer.
    merged.search_history = settings.search_history.clone();

    // An ARL is no longer mandatory: Spotify is an independent source, so the
    // settings must be saveable before (or without) a Deezer login.
    drop(settings);
    let settings_to_save = merged.clone();
    run_blocking(move || settings_to_save.save(&app)).await?;
    *state.settings.lock().await = merged;
    Ok(())
}

#[tauri::command]
pub async fn update_settings(
    updates: serde_json::Map<String, Value>,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if let Some(value) = updates.get("output_dir") {
        let output_dir: String = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        let current_output_dir = state.settings.lock().await.output_dir.clone();
        if output_dir != current_output_dir {
            require_granted_folder(&state, &output_dir).await?;
        }
    }

    let _settings_io = state.settings_io.lock().await;
    let mut settings = state.settings.lock().await.clone();

    for (key, value) in updates {
        match key.as_str() {
            "output_dir" => settings.output_dir = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "quality" => settings.quality = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "folder_structure" => settings.folder_structure = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "custom_folder_template" => settings.custom_folder_template = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "theme" => settings.theme = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "custom_theme" => settings.custom_theme = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "notifications_enabled" => settings.notifications_enabled = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "enable_search_history" => settings.enable_search_history = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "log_track_details" => settings.log_track_details = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "close_to_tray" => settings.close_to_tray = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "skip_existing_files" => settings.skip_existing_files = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "download_delay_seconds" => settings.download_delay_seconds = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "download_speed_limit_kbps" => settings.download_speed_limit_kbps = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "max_concurrent_downloads" => settings.max_concurrent_downloads = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "spotify_enabled" => settings.spotify_enabled = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "spotify_quality" => settings.spotify_quality = serde_json::from_value(value).map_err(|e| e.to_string())?,
            "spotify_client_id" => {
                let id: String = serde_json::from_value(value).map_err(|e| e.to_string())?;
                settings.spotify_client_id = id.trim().to_string();
            }
            "locale" => settings.locale = serde_json::from_value(value).map_err(|e| e.to_string())?,
            _ => return Err(format!("Setting '{}' cannot be updated individually", key)),
        }
    }

    let settings_to_save = settings.clone();
    run_blocking(move || settings_to_save.save(&app)).await?;
    *state.settings.lock().await = settings;
    Ok(())
}

#[tauri::command]
pub async fn pick_folder(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title("Choose download folder")
        .pick_folder(move |folder_path| {
            let _ = tx.send(folder_path.map(|p| p.to_string()));
        });

    let path = rx.await.unwrap_or(None);
    if let Some(ref path) = path {
        grant_folder(&state, path).await?;
    }
    Ok(path)
}

#[tauri::command]
pub async fn add_search_history(
    query: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let _settings_io = state.settings_io.lock().await;
    let mut settings = state.settings.lock().await.clone();
    
    if !settings.enable_search_history {
        return Ok(());
    }
    
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(());
    }

    // Reject unreasonably long queries to prevent oversized settings file
    if query.len() > 500 {
        return Ok(());
    }

    // Remove duplicate if exists
    settings.search_history.retain(|q| q != &query);
    
    // Add to front
    settings.search_history.insert(0, query);
    
    // Keep only last 20 searches
    if settings.search_history.len() > 20 {
        settings.search_history.truncate(20);
    }
    
    let settings_to_save = settings.clone();
    run_blocking(move || settings_to_save.save(&app)).await?;
    *state.settings.lock().await = settings;
    Ok(())
}

#[tauri::command]
pub async fn get_search_history(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let settings = state.settings.lock().await;
    Ok(settings.search_history.clone())
}

#[tauri::command]
pub async fn clear_search_history(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let _settings_io = state.settings_io.lock().await;
    let mut settings = state.settings.lock().await.clone();
    settings.search_history.clear();
    let settings_to_save = settings.clone();
    run_blocking(move || settings_to_save.save(&app)).await?;
    *state.settings.lock().await = settings;
    Ok(())
}
