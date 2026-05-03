use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: u64,
    pub name: String,
    pub image: Option<String>,
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
    pub status: String,
}

/// Tag data read from an existing MP3 or FLAC file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTagData {
    pub file_path: String,
    pub format: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<i32>,
    pub track: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc: Option<u32>,
    pub total_discs: Option<u32>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub comment: Option<String>,
    /// Base64-encoded cover image for display in the UI.
    pub cover_data: Option<String>,
    pub cover_mime: Option<String>,
}

/// Tag fields to write back to an audio file.
/// `None` means "do not change this field".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteTagData {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<i32>,
    pub track: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc: Option<u32>,
    pub total_discs: Option<u32>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub comment: Option<String>,
    /// Path to a new cover image file. `None` = keep existing cover.
    pub new_cover_path: Option<String>,
    /// If true and `new_cover_path` is None, remove the existing cover.
    pub remove_cover: bool,
}
