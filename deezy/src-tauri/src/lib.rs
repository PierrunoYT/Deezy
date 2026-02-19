mod commands;
mod deezer;
mod settings;

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub client: Arc<Mutex<Option<deezer::DeezerClient>>>,
    pub settings: Arc<Mutex<settings::Settings>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            client: Arc::new(Mutex::new(None)),
            settings: Arc::new(Mutex::new(settings::Settings::default())),
        })
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::search_tracks,
            commands::search_albums,
            commands::get_album_tracks,
            commands::download_track,
            commands::get_settings,
            commands::save_settings,
            commands::pick_folder,
            commands::save_download_history,
            commands::load_download_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
