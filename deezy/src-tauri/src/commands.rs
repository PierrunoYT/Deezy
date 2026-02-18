use crate::deezer::download;
use crate::deezer::models::SearchResult;
use crate::deezer::DeezerClient;
use crate::settings::Settings;
use crate::AppState;
use serde_json::Value;
use tauri::AppHandle;

#[tauri::command]
pub async fn login(
    arl: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Value, String> {
    let client = DeezerClient::new(&arl).await?;
    let user = serde_json::to_value(&client.user).map_err(|e| e.to_string())?;

    *state.client.lock().await = Some(client);

    {
        let mut settings = state.settings.lock().await;
        settings.arl = arl;
        settings.save(&app)?;
    }

    Ok(user)
}

#[tauri::command]
pub async fn search_tracks(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let lock = state.client.lock().await;
    let client = lock
        .as_ref()
        .ok_or("Not logged in. Set your ARL token in Settings.")?;
    client.search_tracks(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn download_track(
    trackId: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    eprintln!("Download track command called for trackId: {}", trackId);
    
    // Get or recreate the client
    let (mut client, output_dir, quality, arl) = {
        let lock = state.client.lock().await;
        let settings = state.settings.lock().await;
        eprintln!("Settings - output_dir: {}, quality: {}", settings.output_dir, settings.quality);
        
        let client = if let Some(c) = lock.as_ref() {
            c.clone()
        } else {
            return Err("Not logged in. Please set your ARL token in Settings.".to_string());
        };
        
        (
            client,
            settings.output_dir.clone(),
            settings.quality.clone(),
            settings.arl.clone(),
        )
    };
    
    // If token is empty or invalid, try to refresh the client
    if client.token.is_empty() && !arl.is_empty() {
        eprintln!("Token is empty, recreating client...");
        match DeezerClient::new(&arl).await {
            Ok(new_client) => {
                client = new_client.clone();
                *state.client.lock().await = Some(new_client);
                eprintln!("Client recreated successfully");
            }
            Err(e) => {
                return Err(format!("Failed to refresh session: {}", e));
            }
        }
    }

    eprintln!("Starting download to: {}", output_dir);
    let mut result = download::download_track(&client, &trackId, &output_dir, &quality, &app).await;
    
    // If we get a CSRF error, try to refresh the client and retry once
    if let Err(ref e) = result {
        if e.contains("CSRF") || e.contains("token") {
            eprintln!("CSRF error detected, refreshing client and retrying...");
            match DeezerClient::new(&arl).await {
                Ok(new_client) => {
                    client = new_client.clone();
                    *state.client.lock().await = Some(new_client);
                    eprintln!("Client refreshed, retrying download...");
                    result = download::download_track(&client, &trackId, &output_dir, &quality, &app).await;
                }
                Err(refresh_err) => {
                    eprintln!("Failed to refresh client: {}", refresh_err);
                    return Err(format!("Session expired. Please go to Settings and log in again. Error: {}", e));
                }
            }
        }
    }
    
    match &result {
        Ok(path) => eprintln!("Download successful: {}", path),
        Err(e) => eprintln!("Download failed: {}", e),
    }
    
    result
}

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Settings, String> {
    let loaded = Settings::load(&app)?;
    let mut settings = state.settings.lock().await;
    *settings = loaded.clone();
    Ok(loaded)
}

#[tauri::command]
pub async fn save_settings(
    new_settings: Settings,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut settings = state.settings.lock().await;
    *settings = new_settings.clone();
    settings.save(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title("Choose download folder")
        .pick_folder(move |folder_path| {
            let _ = tx.send(folder_path.map(|p| p.to_string()));
        });

    match rx.await {
        Ok(path) => Ok(path),
        Err(_) => Ok(None),
    }
}
