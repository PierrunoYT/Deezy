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

/// Longest tooltip in UTF-16 units. Windows keeps tray tooltips in a 128-unit
/// buffer that also needs room for the terminating NUL.
const MAX_TOOLTIP_UNITS: usize = 127;

#[tauri::command]
pub async fn set_tray_tooltip(
    tooltip: String,
    app: AppHandle,
) -> Result<(), String> {
    app_tray::set_tray_tooltip(&app, cap_tooltip(&tooltip))
        .map_err(|e| e.to_string())
}

/// Cuts a tooltip to [`MAX_TOOLTIP_UNITS`] without splitting a character.
fn cap_tooltip(tooltip: &str) -> &str {
    let mut units = 0;
    for (index, c) in tooltip.char_indices() {
        units += c.len_utf16();
        if units > MAX_TOOLTIP_UNITS {
            return &tooltip[..index];
        }
    }
    tooltip
}

#[cfg(test)]
mod tests {
    use super::{cap_tooltip, MAX_TOOLTIP_UNITS};

    #[test]
    fn tooltips_fit_the_windows_buffer_without_splitting_characters() {
        assert_eq!(cap_tooltip("Wavekeep"), "Wavekeep");
        let long = "x".repeat(200);
        assert_eq!(cap_tooltip(&long).len(), MAX_TOOLTIP_UNITS);
        // Each emoji takes two UTF-16 units; the one that would straddle the limit is dropped.
        let emoji = "🎵".repeat(70);
        let capped = cap_tooltip(&emoji);
        assert_eq!(capped.chars().count(), 63);
        assert!(capped.encode_utf16().count() <= MAX_TOOLTIP_UNITS);
    }
}
