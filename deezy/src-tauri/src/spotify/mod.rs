//! Spotify download engine.
//!
//! Catalog (search, albums, playlists) goes through Spotify's Web API, while
//! audio comes from the librespot AP session — the same path the official
//! desktop client uses. Both share one OAuth access token.

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

pub fn is_spotify_track(key: &str) -> bool {
    key.starts_with(KEY_PREFIX)
}

pub fn bare_track_id(key: &str) -> &str {
    key.strip_prefix(KEY_PREFIX).unwrap_or(key)
}

pub fn track_key(id: &str) -> String {
    format!("{}{}", KEY_PREFIX, id)
}
