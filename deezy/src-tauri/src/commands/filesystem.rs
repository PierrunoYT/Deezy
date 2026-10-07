use super::*;

#[tauri::command]
pub async fn show_in_folder(
    file_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path = require_audio_file(&state, &file_path).await?;

    #[cfg(target_os = "windows")]
    {
        // Open the parent directory directly; this is more reliable than /select
        // across path formats and still lands users in the song folder.
        let target_dir = path
            .parent()
            .ok_or("Failed to resolve file parent directory")?
            .to_path_buf();

        if !target_dir.exists() {
            return Err("Target directory not found".to_string());
        }

        // canonicalize returns verbatim paths, which Explorer doesn't accept:
        // \\?\UNC\server\share -> \\server\share, and \\?\C:\dir -> C:\dir.
        let mut windows_dir = target_dir.to_string_lossy().replace('/', "\\");
        if let Some(stripped) = windows_dir.strip_prefix(r"\\?\UNC\") {
            windows_dir = format!(r"\\{}", stripped);
        } else if let Some(stripped) = windows_dir.strip_prefix(r"\\?\") {
            windows_dir = stripped.to_string();
        }

        Command::new("explorer")
            .arg(windows_dir)
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
    let parsed = Url::parse(url.trim()).map_err(|_| "Invalid URL".to_string())?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("Invalid Deezer URL: only http/https are supported".to_string());
    }

    let host = parsed
        .host_str()
        .map(|h| h.to_ascii_lowercase())
        .ok_or("Invalid Deezer URL host".to_string())?;
    if host != "deezer.com" && host != "www.deezer.com" {
        return Err("Invalid Deezer URL: host must be deezer.com".to_string());
    }

    let segments: Vec<_> = parsed
        .path_segments()
        .ok_or("Invalid Deezer URL path".to_string())?
        .filter(|segment| !segment.is_empty())
        .collect();

    let (content_type, id) = match segments.as_slice() {
        [kind, id] if is_supported_deezer_kind(kind) => (*kind, *id),
        [locale, kind, id] if is_locale_segment(locale) && is_supported_deezer_kind(kind) => (*kind, *id),
        _ => return Err("Invalid Deezer URL format".to_string()),
    };

    if !id.chars().all(|c| c.is_ascii_digit()) {
        return Err("Invalid Deezer URL: expected numeric identifier".to_string());
    }

    Ok(serde_json::json!({
        "type": content_type,
        "id": id
    }))
}

fn is_supported_deezer_kind(segment: &str) -> bool {
    matches!(segment, "track" | "album" | "artist" | "playlist")
}

fn is_locale_segment(segment: &str) -> bool {
    segment.len() == 2 && segment.chars().all(|c| c.is_ascii_alphabetic())
}
