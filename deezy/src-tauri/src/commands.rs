use crate::deezer::download;
use crate::deezer::models::{AlbumResult, ArtistResult, DownloadResult, PlaylistResult, SearchResult};
use crate::deezer::DeezerClient;
use crate::settings::Settings;
use crate::themes;
use crate::tray;
use crate::AppState;
use serde_json::Value;
use std::process::Command;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use regex::Regex;

#[tauri::command]
pub async fn save_download_history(history: Vec<serde_json::Value>, app: AppHandle) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("download_history.json");
    let data = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_download_history(app: AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = dir.join("download_history.json");
    if path.exists() {
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    } else {
        Ok(vec![])
    }
}

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
pub async fn auto_login(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Option<Value>, String> {
    let settings = Settings::load(&app)?;
    if settings.arl.trim().is_empty() {
        return Ok(None);
    }

    let client = DeezerClient::new(&settings.arl).await?;
    let user = serde_json::to_value(&client.user).map_err(|e| e.to_string())?;

    *state.client.lock().await = Some(client);
    *state.settings.lock().await = settings;

    Ok(Some(user))
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
pub async fn search_albums(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AlbumResult>, String> {
    let lock = state.client.lock().await;
    let client = lock
        .as_ref()
        .ok_or("Not logged in. Set your ARL token in Settings.")?;
    client.search_albums(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_album_tracks(
    albumId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let lock = state.client.lock().await;
    let client = lock
        .as_ref()
        .ok_or("Not logged in. Set your ARL token in Settings.")?;
    client.get_album_tracks(&albumId).await
}

#[tauri::command]
pub async fn search_artists(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ArtistResult>, String> {
    let lock = state.client.lock().await;
    let client = lock
        .as_ref()
        .ok_or("Not logged in. Set your ARL token in Settings.")?;
    client.search_artists(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_artist_albums(
    artistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AlbumResult>, String> {
    let lock = state.client.lock().await;
    let client = lock
        .as_ref()
        .ok_or("Not logged in. Set your ARL token in Settings.")?;
    client.get_artist_albums(&artistId).await
}

#[tauri::command]
pub async fn search_playlists(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PlaylistResult>, String> {
    let lock = state.client.lock().await;
    let client = lock
        .as_ref()
        .ok_or("Not logged in. Set your ARL token in Settings.")?;
    client.search_playlists(&query, 20).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_playlist_tracks(
    playlistId: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let lock = state.client.lock().await;
    let client = lock
        .as_ref()
        .ok_or("Not logged in. Set your ARL token in Settings.")?;
    client.get_playlist_tracks(&playlistId).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn download_track(
    trackId: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<DownloadResult, String> {
    // Get or recreate the client
    let (mut client, output_dir, quality, folder_structure, arl) = {
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
            settings.arl.clone(),
        )
    };
    
    // If token is empty or invalid, try to refresh the client
    if client.token.is_empty() && !arl.is_empty() {
        match DeezerClient::new(&arl).await {
            Ok(new_client) => {
                client = new_client.clone();
                *state.client.lock().await = Some(new_client);
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
        &trackId,
        &output_dir,
        &effective_quality,
        &folder_structure,
        &app,
    )
    .await;
    
    // If we get a CSRF error, try to refresh the client and retry once
    if let Err(ref e) = result {
        if e.contains("CSRF") || e.contains("token") {
            match DeezerClient::new(&arl).await {
                Ok(new_client) => {
                    client = new_client.clone();
                    *state.client.lock().await = Some(new_client);
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
                        &trackId,
                        &output_dir,
                        &retry_quality,
                        &folder_structure,
                        &app,
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

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Settings, String> {
    let loaded = Settings::load(&app)?;
    {
        let mut settings = state.settings.lock().await;
        *settings = loaded.clone();
    }

    // Never return ARL to the renderer process.
    let mut safe = loaded;
    safe.arl = String::new();
    Ok(safe)
}

#[tauri::command]
pub async fn save_settings(
    new_settings: Settings,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut settings = state.settings.lock().await;
    let mut merged = new_settings.clone();

    // Allow non-auth settings updates without exposing ARL to the renderer.
    if merged.arl.trim().is_empty() {
        merged.arl = settings.arl.clone();
    }

    if merged.arl.trim().is_empty() {
        return Err("ARL token is required".to_string());
    }

    merged.save(&app)?;
    *settings = merged;
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

#[tauri::command]
pub async fn add_search_history(
    query: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut settings = state.settings.lock().await;
    
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
    
    settings.save(&app)?;
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
    let mut settings = state.settings.lock().await;
    settings.search_history.clear();
    settings.save(&app)?;
    Ok(())
}

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
    tray::update_tray_menu(&app, downloads_active, downloads_paused)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_tray_tooltip(
    tooltip: String,
    app: AppHandle,
) -> Result<(), String> {
    tray::set_tray_tooltip(&app, &tooltip)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_download_history(
    history: Vec<serde_json::Value>,
    format: String,
    app: AppHandle,
) -> Result<String, String> {
    let extension = match format.as_str() {
        "csv" => "csv",
        "json" => "json",
        _ => return Err("Invalid format. Use 'csv' or 'json'.".to_string()),
    };

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title("Export Download History")
        .add_filter(format.to_uppercase(), &[extension])
        .set_file_name(format!("deezy_download_history.{}", extension))
        .save_file(move |file_path| {
            let _ = tx.send(
                file_path.and_then(|p| p.as_path().map(|path| path.to_string_lossy().to_string())),
            );
        });

    let file_path = match rx.await {
        Ok(Some(path)) => path,
        Ok(None) => return Err("Export cancelled".to_string()),
        Err(_) => return Err("Failed to get file path".to_string()),
    };

    let content = if format == "csv" {
        generate_csv(&history)?
    } else {
        serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?
    };

    std::fs::write(&file_path, content).map_err(|e| e.to_string())?;
    Ok(file_path)
}

fn generate_csv(history: &[serde_json::Value]) -> Result<String, String> {
    let mut csv = String::from("Title,Artist,Album,Status,Progress,Timestamp,File Path,Error Message\n");

    for item in history {
        let title = sanitize_csv_field(item["title"].as_str().unwrap_or(""));
        let artist = sanitize_csv_field(item["artist"].as_str().unwrap_or(""));
        let album = sanitize_csv_field(item["album"].as_str().unwrap_or(""));
        let status = sanitize_csv_field(item["status"].as_str().unwrap_or(""));
        let percent = format!("{:.1}%", item["percent"].as_f64().unwrap_or(0.0));
        let timestamp = sanitize_csv_field(item["timestamp"].as_str().unwrap_or(""));
        let file_path = sanitize_csv_field(item["filePath"].as_str().unwrap_or(""));
        let error_msg = sanitize_csv_field(item["errorMsg"].as_str().unwrap_or(""));

        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            title, artist, album, status, percent, timestamp, file_path, error_msg
        ));
    }

    Ok(csv)
}

fn sanitize_csv_field(value: &str) -> String {
    let escaped = value.replace("\"", "\"\"");
    if let Some(first) = escaped.chars().next() {
        if matches!(first, '=' | '+' | '-' | '@' | '\t' | '\r') {
            return format!("'{}", escaped);
        }
    }
    escaped
}

#[tauri::command]
pub async fn list_custom_themes(app: AppHandle) -> Result<Vec<String>, String> {
    themes::list_custom_themes(&app)
}

#[tauri::command]
pub async fn load_custom_theme(theme_name: String, app: AppHandle) -> Result<themes::CustomTheme, String> {
    themes::load_custom_theme(&app, &theme_name)
}

#[tauri::command]
pub async fn save_custom_theme(theme: themes::CustomTheme, app: AppHandle) -> Result<(), String> {
    themes::save_custom_theme(&app, &theme)
}

#[tauri::command]
pub async fn delete_custom_theme(theme_name: String, app: AppHandle) -> Result<(), String> {
    themes::delete_custom_theme(&app, &theme_name)
}

#[tauri::command]
pub async fn export_current_theme(
    theme_name: String,
    author: Option<String>,
    description: Option<String>,
    is_light: bool,
) -> Result<themes::CustomTheme, String> {
    Ok(themes::export_current_theme(theme_name, author, description, is_light))
}

#[tauri::command]
pub async fn import_theme_file(app: AppHandle) -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title("Import Theme File")
        .add_filter("JSON", &["json"])
        .pick_file(move |file_path| {
            let _ = tx.send(file_path.map(|p| p.to_string()));
        });

    let file_path = match rx.await {
        Ok(Some(path)) => path,
        Ok(None) => return Err("Import cancelled".to_string()),
        Err(_) => return Err("Failed to get file path".to_string()),
    };

    let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let theme: themes::CustomTheme = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    theme.validate()?;
    
    themes::save_custom_theme(&app, &theme)?;
    
    Ok(theme.name.clone())
}

#[tauri::command]
pub async fn create_example_themes(app: AppHandle) -> Result<(), String> {
    themes::create_example_themes(&app)
}

#[tauri::command]
pub async fn show_in_folder(file_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&file_path);

    if !path.exists() {
        return Err("File not found".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let absolute = if path.is_absolute() {
            path.clone()
        } else {
            std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?
                .join(&path)
        };
        let mut windows_path = absolute.to_string_lossy().replace('/', "\\");
        if let Some(stripped) = windows_path.strip_prefix(r"\\?\") {
            windows_path = stripped.to_string();
        }

        Command::new("explorer")
            .arg(format!("/select,{}", windows_path))
            .spawn()
            .map_err(|e| format!("Failed to open Explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to reveal file in Finder: {}", e))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path
            .parent()
            .ok_or("Failed to resolve file parent directory")?;

        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn parse_deezer_url(url: String) -> Result<Value, String> {
    lazy_static::lazy_static! {
        static ref DEEZER_URL_REGEX: Regex = Regex::new(
            r"https?://(?:www\.)?deezer\.com/(?:[a-z]{2}/)?(track|album|artist|playlist)/(\d+)"
        ).unwrap();
    }

    if let Some(captures) = DEEZER_URL_REGEX.captures(&url) {
        let content_type = captures.get(1).unwrap().as_str();
        let id = captures.get(2).unwrap().as_str();
        
        Ok(serde_json::json!({
            "type": content_type,
            "id": id
        }))
    } else {
        Err("Invalid Deezer URL format".to_string())
    }
}
