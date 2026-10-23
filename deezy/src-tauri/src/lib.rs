mod commands;
mod deezer;
mod logging;
mod migration;
mod paths;
mod settings;
mod spotify;
mod themes;
mod tray;

use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter, Listener, Manager};

/// Event that asks the renderer to save its state and call `exit_app`. The
/// close button, the tray's Quit item and an OS-level quit all send it.
const EXIT_REQUESTED_EVENT: &str = "app-exit-requested";
/// How long the renderer gets to call `exit_app` before the backend exits on
/// its own (it pauses downloads for up to 5 s, then saves the history).
const EXIT_FALLBACK_DELAY: Duration = Duration::from_secs(15);

#[derive(Default)]
pub struct FileGrants {
    pub audio: HashSet<PathBuf>,
    pub images: HashSet<PathBuf>,
    pub folders: HashSet<PathBuf>,
}

pub struct AppState {
    pub client: Arc<Mutex<Option<deezer::DeezerClient>>>,
    pub spotify: Arc<Mutex<Option<Arc<spotify::SpotifyClient>>>>,
    pub settings: Arc<Mutex<settings::Settings>>,
    pub settings_io: Arc<Mutex<()>>,
    pub tray_state: tray::TrayState,
    pub download_cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    pub file_grants: Arc<Mutex<FileGrants>>,
    pub bandwidth: Arc<deezer::bandwidth::BandwidthLimiter>,
}

pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        let thread = std::thread::current();
        logging::error("app", format!(
            "PANIC on thread '{}': {}\n{}",
            thread.name().unwrap_or("unnamed"),
            info,
            std::backtrace::Backtrace::force_capture()
        ));
    }));
    logging::info(
        "app",
        format!(
            "Wavekeep {} starting on {} {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    );

    tauri::Builder::default()
        // Must be registered first. A second launch would load its own copy of
        // settings and overwrite newer changes made by this one, so it focuses
        // the running window instead (which may be hidden in the tray).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(navigation_guard())
        .manage(AppState {
            client: Arc::new(Mutex::new(None)),
            spotify: Arc::new(Mutex::new(None)),
            settings: Arc::new(Mutex::new(settings::Settings::default())),
            settings_io: Arc::new(Mutex::new(())),
            tray_state: tray::TrayState::new(),
            download_cancellations: Arc::new(Mutex::new(HashMap::new())),
            file_grants: Arc::new(Mutex::new(FileGrants::default())),
            bandwidth: Arc::new(deezer::bandwidth::BandwidthLimiter::new()),
        })
        .setup(|app| {
            // The log folder is known from here on; earlier lines are flushed.
            if let Ok(dir) = app.path().app_log_dir() {
                logging::init(dir);
            }

            // Before anything reads settings or credentials.
            migration::run(app.handle());

            // Create system tray
            if let Err(e) = tray::create_tray(app.handle()) {
                logging::error("app", format!("Failed to create system tray: {}", e));
            }

            // However the exit was requested, stop new downloads and make sure
            // the app exits even if the renderer never calls `exit_app`.
            let handle = app.handle().clone();
            app.listen_any(EXIT_REQUESTED_EVENT, move |_| arm_exit_fallback(&handle));

            // Handle window close event
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // The window never closes directly: it is either hidden
                        // or the renderer first saves its pending download
                        // history. Which one depends on a setting, read in a
                        // task so a busy settings lock neither blocks the main
                        // thread nor turns "hide to tray" into an exit.
                        api.prevent_close();
                        let window = window_clone.clone();
                        tauri::async_runtime::spawn(async move {
                            let app_handle = window.app_handle().clone();
                            let state = app_handle.state::<AppState>();
                            let close_to_tray = state.settings.lock().await.close_to_tray;
                            if close_to_tray {
                                let _ = window.hide();
                            } else {
                                let _ = app_handle.emit(EXIT_REQUESTED_EVENT, ());
                            }
                        });
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
            commands::get_loved_tracks,
            commands::get_user_playlists,
            commands::spotify_login,
            commands::spotify_logout,
            commands::spotify_status,
            commands::search_spotify_tracks,
            commands::search_spotify_albums,
            commands::search_spotify_artists,
            commands::search_spotify_playlists,
            commands::get_spotify_track,
            commands::get_spotify_album_tracks,
            commands::get_spotify_artist_albums,
            commands::get_spotify_playlist_tracks,
            commands::get_spotify_loved_tracks,
            commands::get_spotify_user_playlists,
            commands::download_track,
            commands::cancel_download,
            commands::get_settings,
            commands::get_arl_storage_status,
            commands::save_settings,
            commands::update_settings,
            commands::pick_folder,
            commands::save_download_history,
            commands::load_download_history,
            commands::exit_app,
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
            commands::set_tray_labels,
            commands::write_log,
            commands::read_recent_log,
            commands::delete_log,
            commands::open_log_folder,
            commands::parse_deezer_url,
            commands::pick_audio_file,
            commands::pick_cover_image,
            commands::read_image_as_data_url,
            commands::read_file_tags,
            commands::write_file_tags,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // An exit without a code comes from the OS (macOS Cmd+Q, the Dock,
            // the app menu) rather than from `exit_app`. Route it through the
            // normal exit so downloads are cancelled and their files removed.
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = &event {
                api.prevent_exit();
                let _ = app.emit(EXIT_REQUESTED_EVENT, ());
            }
        });
}

/// Starts the backend's exit fallback, once: new downloads are refused at
/// once, and if the renderer has not called `exit_app` within
/// `EXIT_FALLBACK_DELAY` (hung or crashed WebView), the backend runs the same
/// cleanup and exits itself.
fn arm_exit_fallback(app: &AppHandle) {
    static ARMED: AtomicBool = AtomicBool::new(false);

    commands::begin_shutdown();
    if ARMED.swap(true, Ordering::SeqCst) {
        return;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(EXIT_FALLBACK_DELAY).await;
        if commands::exit_started() {
            return;
        }
        logging::warn(
            "app",
            format!(
                "The window did not finish closing within {} s; exiting from the backend",
                EXIT_FALLBACK_DELAY.as_secs()
            ),
        );
        let state = app.state::<AppState>();
        commands::shut_down_and_exit(state.inner(), &app).await;
    });
}

/// Keeps the WebView on the app's own pages. A script injected into the page
/// could otherwise send what it can read to another site by navigating there.
/// (New windows from `window.open` are already refused, because no handler
/// for them is installed.)
fn navigation_guard<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("navigation-guard")
        .on_navigation(|_, url| {
            let allowed = is_app_url(url);
            if !allowed {
                logging::warn(
                    "app",
                    format!("Blocked navigation to {}://{}", url.scheme(), url.host_str().unwrap_or("")),
                );
            }
            allowed
        })
        .build()
}

/// The app's own origin: `tauri://localhost` (macOS, Linux),
/// `http(s)://tauri.localhost` (Windows) and, in development, the Vite server.
fn is_app_url(url: &tauri::Url) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => {
            url.host_str() == Some("tauri.localhost")
                || (cfg!(dev)
                    && url.scheme() == "http"
                    && url.host_str() == Some("localhost")
                    && url.port() == Some(1420))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_app_url;

    fn allowed(url: &str) -> bool {
        is_app_url(&url.parse().unwrap())
    }

    #[test]
    fn navigation_stays_on_the_app_origin() {
        assert!(allowed("tauri://localhost/"));
        assert!(allowed("http://tauri.localhost/index.html"));
        assert!(allowed("https://tauri.localhost/"));

        assert!(!allowed("https://evil.example/?data=1"));
        assert!(!allowed("https://tauri.localhost.evil.example/"));
        assert!(!allowed("http://user@evil.example/"));
        assert!(!allowed("tauri://evil/"));
        assert!(!allowed("file:///C:/Windows/"));
        assert!(!allowed("data:text/html,hi"));
        assert!(!allowed("javascript:alert(1)"));
        assert!(!allowed("http://localhost:8080/"));
    }
}
