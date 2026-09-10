use super::*;

#[tauri::command]
pub async fn exit_app(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let cancellations = state.download_cancellations.lock().await;
        for flag in cancellations.values() {
            flag.store(true, Ordering::Relaxed);
        }
    }

    // Each download removes its registration only after its temporary/fallback
    // files have been cleaned. Do not force-exit while cleanup is outstanding.
    while !state.download_cancellations.lock().await.is_empty() {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn login(
    arl: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Value, String> {
    let client = DeezerClient::new(&arl).await?;
    let user = serde_json::to_value(&client.user).map_err(|e| e.to_string())?;

    let _settings_io = state.settings_io.lock().await;
    let mut updated_settings = state.settings.lock().await.clone();
    updated_settings.arl = arl;
    let settings_to_save = updated_settings.clone();
    run_blocking(move || settings_to_save.save(&app)).await?;
    *state.settings.lock().await = updated_settings;

    // Do not expose a client backed by an unpersisted credential.
    *state.client.lock().await = Some(client);

    Ok(user)
}

#[tauri::command]
pub async fn auto_login(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Option<Value>, String> {
    let settings_io = state.settings_io.lock().await;
    let settings = run_blocking(move || Settings::load(&app)).await?;
    *state.settings.lock().await = settings.clone();
    let _ = grant_folder(&state, &settings.output_dir).await;
    drop(settings_io);
    if settings.arl.trim().is_empty() {
        return Ok(None);
    }

    let client = DeezerClient::new(&settings.arl).await?;
    let user = serde_json::to_value(&client.user).map_err(|e| e.to_string())?;

    let _settings_io = state.settings_io.lock().await;
    if state.settings.lock().await.arl != settings.arl {
        return Err("Account changed while automatic login was in progress".to_string());
    }
    *state.client.lock().await = Some(client);

    Ok(Some(user))
}
