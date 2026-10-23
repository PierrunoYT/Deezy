//! Spotify download engine.
//!
//! Search starts on Spotify's Web API, with a Pathfinder fallback for track,
//! album, and artist rate limits. Catalog metadata and audio use the librespot
//! AP session — the same path the official desktop client uses. Web API and AP
//! requests share one OAuth access token.

pub mod api;
pub mod ap;
pub mod auth;
pub mod download;
pub mod pathfinder;
pub mod tags;

pub use api::{SpotifyAlbum, SpotifyArtist, SpotifyPlaylist, SpotifyTrack};
pub use auth::SpotifyClient;

/// Track keys handed to the download queue are prefixed so a Spotify ID can
/// never collide with a numeric Deezer ID (or with a Deezer download in flight).
pub const KEY_PREFIX: &str = "spotify:";

const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Prints to stderr and appends to `wavekeep-debug.log` in the temp dir, so the
/// trail survives a crash that closes the console.
pub fn debug_log(message: &str) {
    use std::io::Write;

    eprintln!("[wavekeep] {}", message);
    let path = std::env::temp_dir().join("wavekeep-debug.log");
    // Start over rather than grow forever.
    if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", secs, message);
    }
}

pub fn is_spotify_track(key: &str) -> bool {
    key.starts_with(KEY_PREFIX)
}

pub fn bare_track_id(key: &str) -> &str {
    key.strip_prefix(KEY_PREFIX).unwrap_or(key)
}

pub fn track_key(id: &str) -> String {
    format!("{}{}", KEY_PREFIX, id)
}
