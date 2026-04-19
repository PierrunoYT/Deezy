mod commands;
mod deezer;
mod settings;
mod themes;
mod tray;

use std::sync::Arc;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub client: Arc<Mutex<Option<deezer::DeezerClient>>>,
    pub settings: Arc<Mutex<settings::Settings>>,
    pub tray_state: tray::TrayState,
    pub download_cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            client: Arc::new(Mutex::new(None)),
            settings: Arc::new(Mutex::new(settings::Settings::default())),
            tray_state: tray::TrayState::new(),
            download_cancellations: Arc::new(Mutex::new(HashMap::new())),
        })
        .setup(|app| {
            // Create system tray
            if let Err(e) = tray::create_tray(app.handle()) {
                eprintln!("Failed to create system tray: {}", e);
            }

            // Handle window close event
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Get settings to check if close_to_tray is enabled.
                        // Use try_lock to avoid blocking the main thread; if the
                        // lock is contended, fall through and allow the close.
                        let app_handle = window_clone.app_handle();
                        let state: tauri::State<AppState> = app_handle.state();
                        
                        let close_to_tray = state.settings
                            .try_lock()
                            .map(|s| s.close_to_tray)
                            .unwrap_or(false);

                        if close_to_tray {
                            // Hide window instead of closing
                            let _ = window_clone.hide();
                            api.prevent_close();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::auto_login,
            commands::search_tracks,
            commands::get_track_by_id,
            commands::search_albums,
            commands::search_artists,
            commands::get_album_tracks,
            commands::get_artist_albums,
            commands::search_playlists,
            commands::get_playlist_tracks,
            commands::download_track,
            commands::cancel_download,
            commands::get_settings,
            commands::save_settings,
            commands::pick_folder,
            commands::save_download_history,
            commands::load_download_history,
            commands::export_download_history,
            commands::add_search_history,
            commands::get_search_history,
            commands::clear_search_history,
            commands::update_tray_status,
            commands::set_tray_tooltip,
            commands::list_custom_themes,
            commands::load_custom_theme,
            commands::save_custom_theme,
            commands::delete_custom_theme,
            commands::export_current_theme,
            commands::import_theme_file,
            commands::create_example_themes,
            commands::show_in_folder,
            commands::parse_deezer_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
