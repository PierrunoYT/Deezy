use super::*;

#[tauri::command]
pub async fn update_tray_status(
    downloads_active: bool,
    downloads_paused: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Update tray state
    *state.tray_state.downloads_active.lock().await = downloads_active;
    *state.tray_state.downloads_paused.lock().await = downloads_paused;

    // Update tray menu
    let labels = state.tray_state.labels.lock().await.clone();
    app_tray::update_tray_menu(&app, downloads_active, downloads_paused, &labels)
        .map_err(|e| e.to_string())
}

/// Stores the tray menu texts for the current language and rebuilds the menu.
#[tauri::command]
pub async fn set_tray_labels(
    labels: app_tray::TrayLabels,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let labels = labels.sanitized();
    *state.tray_state.labels.lock().await = labels.clone();
    let active = *state.tray_state.downloads_active.lock().await;
    let paused = *state.tray_state.downloads_paused.lock().await;
    app_tray::update_tray_menu(&app, active, paused, &labels).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_tray_tooltip(
    tooltip: String,
    app: AppHandle,
) -> Result<(), String> {
    app_tray::set_tray_tooltip(&app, &tooltip)
        .map_err(|e| e.to_string())
}
