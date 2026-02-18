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
pub async fn download_track(
    track_id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let (client, output_dir, quality) = {
        let lock = state.client.lock().await;
        let client = lock.as_ref().ok_or("Not logged in")?.clone();
        let settings = state.settings.lock().await;
        (
            client,
            settings.output_dir.clone(),
            settings.quality.clone(),
        )
    };

    download::download_track(&client, &track_id, &output_dir, &quality, &app).await
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
