use super::*;

#[tauri::command]
pub async fn show_in_folder(
    file_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path = require_audio_file(&state, &file_path).await?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

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

        // Explorer splits its arguments on commas, so "D:\Music\AC,DC" would
        // open the wrong folder unless the path is quoted. Windows paths can't
        // contain a quote; refuse one anyway rather than let it end the quoting.
        if windows_dir.contains('"') {
            return Err("Invalid folder path".to_string());
        }
        spawn_and_reap(Command::new("explorer").raw_arg(format!("\"{}\"", windows_dir)))
            .map_err(|e| format!("Failed to open Explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        spawn_and_reap(Command::new("open").arg("-R").arg(&path))
            .map_err(|e| format!("Failed to reveal file in Finder: {}", e))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path
            .parent()
            .ok_or("Failed to resolve file parent directory")?;

        spawn_and_reap(Command::new("xdg-open").arg(parent))
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn parse_deezer_url(url: String) -> Result<Value, String> {
    let trimmed = url.trim();

    // Spotify URIs: `spotify:track:<id>` and friends.
    if let Some(rest) = trimmed.strip_prefix("spotify:") {
        let mut parts = rest.splitn(2, ':');
        let kind = parts.next().unwrap_or_default();
        let id = parts.next().unwrap_or_default();
        if !is_supported_spotify_kind(kind) || !is_spotify_id(id) {
            return Err("Invalid Spotify URI".to_string());
        }
        return Ok(serde_json::json!({
            "provider": "spotify",
            "type": kind,
            "id": id
        }));
    }

    // A bare 22-character Spotify ID is always a track.
    if trimmed.len() == 22 && is_spotify_id(trimmed) {
        return Ok(serde_json::json!({
            "provider": "spotify",
            "type": "track",
            "id": trimmed
        }));
    }

    let parsed = Url::parse(trimmed).map_err(|_| "Invalid URL".to_string())?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("Invalid music URL: only http/https are supported".to_string());
    }

    let host = parsed
        .host_str()
        .map(|h| h.to_ascii_lowercase())
        .ok_or("Invalid music URL host".to_string())?;

    let segments: Vec<_> = parsed
        .path_segments()
        .ok_or("Invalid music URL path".to_string())?
        .filter(|segment| !segment.is_empty())
        .collect();

    if host == "open.spotify.com" || host == "play.spotify.com" {
        let (content_type, id) = match segments.as_slice() {
            [kind, id] if is_supported_spotify_kind(kind) => (*kind, *id),
            // Localised links: /intl-de/track/<id>
            [locale, kind, id]
                if locale.starts_with("intl-") && is_supported_spotify_kind(kind) =>
            {
                (*kind, *id)
            }
            _ => return Err("Invalid Spotify URL format".to_string()),
        };

        if !is_spotify_id(id) {
            return Err("Invalid Spotify URL: expected base62 identifier".to_string());
        }

        return Ok(serde_json::json!({
            "provider": "spotify",
            "type": content_type,
            "id": id
        }));
    }

    if host != "deezer.com" && host != "www.deezer.com" {
        return Err("Invalid music URL: host must be deezer.com or open.spotify.com".to_string());
    }

    let (content_type, id) = match segments.as_slice() {
        [kind, id] if is_supported_deezer_kind(kind) => (*kind, *id),
        [locale, kind, id] if is_locale_segment(locale) && is_supported_deezer_kind(kind) => (*kind, *id),
        _ => return Err("Invalid Deezer URL format".to_string()),
    };

    if !id.chars().all(|c| c.is_ascii_digit()) {
        return Err("Invalid Deezer URL: expected numeric identifier".to_string());
    }

    Ok(serde_json::json!({
        "provider": "deezer",
        "type": content_type,
        "id": id
    }))
}

fn is_supported_spotify_kind(segment: &str) -> bool {
    matches!(segment, "track" | "album" | "artist" | "playlist")
}

fn is_spotify_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_supported_deezer_kind(segment: &str) -> bool {
    matches!(segment, "track" | "album" | "artist" | "playlist")
}

fn is_locale_segment(segment: &str) -> bool {
    segment.len() == 2 && segment.chars().all(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::parse_deezer_url;

    #[tokio::test]
    async fn parses_spotify_track_urls() {
        let parsed = parse_deezer_url(
            "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC?si=abc".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(parsed["provider"], "spotify");
        assert_eq!(parsed["type"], "track");
        assert_eq!(parsed["id"], "4uLU6hMCjMI75M1A2tKUQC");
    }

    #[tokio::test]
    async fn parses_spotify_uris_and_bare_ids() {
        let uri = parse_deezer_url("spotify:album:4aawyAB9vmqN3uQ7FjRGTy".to_string())
            .await
            .unwrap();
        assert_eq!(uri["type"], "album");
        assert_eq!(uri["provider"], "spotify");

        let bare = parse_deezer_url("4uLU6hMCjMI75M1A2tKUQC".to_string()).await.unwrap();
        assert_eq!(bare["type"], "track");
    }

    #[tokio::test]
    async fn parses_localised_spotify_links() {
        let parsed = parse_deezer_url(
            "https://open.spotify.com/intl-de/playlist/37i9dQZF1DXcBWIGoYBM5M".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(parsed["provider"], "spotify");
        assert_eq!(parsed["type"], "playlist");
    }

    #[tokio::test]
    async fn keeps_parsing_deezer_urls() {
        let parsed = parse_deezer_url("https://www.deezer.com/en/album/302127".to_string())
            .await
            .unwrap();
        assert_eq!(parsed["provider"], "deezer");
        assert_eq!(parsed["type"], "album");
        assert_eq!(parsed["id"], "302127");
    }

    #[tokio::test]
    async fn rejects_unsupported_hosts() {
        assert!(parse_deezer_url("https://example.com/track/1".to_string()).await.is_err());
        assert!(parse_deezer_url("spotify:episode:abc".to_string()).await.is_err());
    }
}
