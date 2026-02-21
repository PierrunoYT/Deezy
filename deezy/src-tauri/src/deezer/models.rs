use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: u64,
    pub name: String,
    pub image: String,
    pub is_free_account: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub artist_id: u64,
    pub album: String,
    pub duration: u64,
    pub cover_small: String,
    pub cover_medium: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumResult {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub artist_id: u64,
    pub cover_small: String,
    pub cover_medium: String,
    pub nb_tracks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistResult {
    pub id: u64,
    pub name: String,
    pub picture_small: String,
    pub picture_medium: String,
    pub nb_album: u64,
    pub nb_fan: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistResult {
    pub id: u64,
    pub title: String,
    pub creator: String,
    pub cover_small: String,
    pub cover_medium: String,
    pub nb_tracks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub track_id: String,
    pub title: String,
    pub percent: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub file_path: String,
    pub requested_quality: String,
    pub actual_quality: String,
}
