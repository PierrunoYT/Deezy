use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::auth::SpotifyClient;
use super::track_key;
use super::{ap, pathfinder};

const API: &str = "https://api.spotify.com/v1";

/// Longest `Retry-After` worth sleeping through before giving up.
const MAX_RETRY_WAIT_SECS: u64 = 5;

#[derive(Debug, Clone, Serialize)]
pub struct SpotifyTrack {
    /// `spotify:<base62>` so the download queue can route it without a flag.
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    pub album: String,
    pub album_id: String,
    /// Seconds, matching the Deezer `Track` shape.
    pub duration: u64,
    pub cover_small: String,
    pub cover_medium: String,
    pub preview: Option<String>,
    pub track_number: u32,
    pub disc_number: u32,
    pub release_date: Option<String>,
    pub isrc: Option<String>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotifyAlbum {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    pub cover_small: String,
    pub cover_medium: String,
    pub nb_tracks: u64,
    pub release_date: Option<String>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotifyArtist {
    pub id: String,
    pub name: String,
    pub picture_small: String,
    pub picture_medium: String,
    pub nb_album: u64,
    pub nb_fan: u64,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub cover_small: String,
    pub cover_medium: String,
    pub nb_tracks: u64,
    pub provider: String,
}

impl SpotifyClient {
    async fn get_json(&self, url: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let mut refreshed = false;
        let mut throttled = 0;
        loop {
            let token = if refreshed {
                self.refresh_token().await?
            } else {
                self.access_token().await?
            };

            let response = self
                .http
                .get(url)
                .bearer_auth(&token)
                .query(query)
                .send()
                .await
                .map_err(|e| format!("Spotify request failed: {}", e))?;

            let status = response.status();
            // Spotify's 429 carries the wait in seconds. Short waits are worth
            // retrying; a long one would freeze the UI, so it fails at once.
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1)
                .max(1);
            let body: Value = response
                .json()
                .await
                .map_err(|e| format!("Spotify response parse failed: {}", e))?;

            if status.as_u16() == 401 && !refreshed {
                refreshed = true;
                continue;
            }

            if status.as_u16() == 429 {
                if throttled < 2 && retry_after <= MAX_RETRY_WAIT_SECS {
                    throttled += 1;
                    tokio::time::sleep(Duration::from_secs(retry_after)).await;
                    continue;
                }
                return Err(rate_limit_message());
            }

            if !status.is_success() {
                return Err(format!(
                    "Spotify API error ({}): {}",
                    status.as_u16(),
                    api_error(&body)
                ));
            }

            return Ok(body);
        }
    }

    /// Follows Spotify's `next` links so long albums/playlists are complete.
    async fn get_paged(&self, url: &str, limit: usize) -> Result<Vec<Value>, String> {
        let mut items = Vec::new();
        let mut next = Some(url.to_string());
        let mut pages = 0;

        while let Some(page_url) = next {
            if items.len() >= limit || pages >= 50 {
                break;
            }
            let body = self.get_json(&page_url, &[]).await?;
            if let Some(page_items) = body["items"].as_array() {
                items.extend(page_items.iter().cloned());
            }
            next = body["next"].as_str().map(str::to_string);
            pages += 1;
        }

        Ok(items)
    }

    pub async fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<SpotifyTrack>, String> {
        match self.search_tracks_web(query, limit).await {
            Err(error) if is_rate_limited(&error) => {
                match pathfinder::search_track_uris(self, query, limit).await {
                    Ok(uris) => ap::load_tracks(&self.session, uris).await,
                    Err(fallback) => Err(format!("{} Fallback: {}", error, fallback)),
                }
            }
            result => result,
        }
    }

    async fn search_tracks_web(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SpotifyTrack>, String> {
        let body = self
            .get_json(
                &format!("{}/search", API),
                &[
                    ("q", query.to_string()),
                    ("type", "track".to_string()),
                    ("limit", limit.to_string()),
                ],
            )
            .await?;

        Ok(body["tracks"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|track| track_from_value(track, None))
            .collect())
    }

    pub async fn search_albums(&self, query: &str, limit: u32) -> Result<Vec<SpotifyAlbum>, String> {
        match self.search_albums_web(query, limit).await {
            Err(error) if is_rate_limited(&error) => {
                match pathfinder::search_album_uris(self, query, limit).await {
                    Ok(uris) => Ok(ap::albums_from_uris(&self.session, uris).await),
                    Err(fallback) => Err(format!("{} Fallback: {}", error, fallback)),
                }
            }
            result => result,
        }
    }

    async fn search_albums_web(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SpotifyAlbum>, String> {
        let body = self
            .get_json(
                &format!("{}/search", API),
                &[
                    ("q", query.to_string()),
                    ("type", "album".to_string()),
                    ("limit", limit.to_string()),
                ],
            )
            .await?;

        Ok(body["albums"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(album_from_value)
            .collect())
    }

    pub async fn search_artists(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SpotifyArtist>, String> {
        match self.search_artists_web(query, limit).await {
            Err(error) if is_rate_limited(&error) => {
                match pathfinder::search_artist_uris(self, query, limit).await {
                    Ok(uris) => Ok(ap::artists_from_uris(&self.session, uris).await),
                    Err(fallback) => Err(format!("{} Fallback: {}", error, fallback)),
                }
            }
            result => result,
        }
    }

    async fn search_artists_web(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SpotifyArtist>, String> {
        let body = self
            .get_json(
                &format!("{}/search", API),
                &[
                    ("q", query.to_string()),
                    ("type", "artist".to_string()),
                    ("limit", limit.to_string()),
                ],
            )
            .await?;

        Ok(body["artists"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(artist_from_value)
            .collect())
    }

    pub async fn search_playlists(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SpotifyPlaylist>, String> {
        let body = self
            .get_json(
                &format!("{}/search", API),
                &[
                    ("q", query.to_string()),
                    ("type", "playlist".to_string()),
                    ("limit", limit.to_string()),
                ],
            )
            .await?;

        Ok(body["playlists"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(playlist_from_value)
            .collect())
    }

    pub async fn user_playlists(&self) -> Result<Vec<SpotifyPlaylist>, String> {
        let items = self
            .get_paged(&format!("{}/me/playlists?limit=50", API), 10000)
            .await?;

        Ok(items.iter().filter_map(playlist_from_value).collect())
    }
}

const RATE_LIMIT_MARKER: &str = "rate limit reached (429)";

fn is_rate_limited(error: &str) -> bool {
    error.contains(RATE_LIMIT_MARKER)
}

fn rate_limit_message() -> String {
    "Spotify rate limit reached (429). The default client ID is shared by every \
     librespot install, so its Web API quota can be exhausted. Wait a moment \
     and try again."
        .to_string()
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;

    #[test]
    fn detects_its_own_rate_limit_message_only() {
        assert!(is_rate_limited(&rate_limit_message()));
        assert!(!is_rate_limited("Spotify API error (500): boom"));
    }
}

fn api_error(body: &Value) -> String {
    body["error"]["message"]
        .as_str()
        .or_else(|| body["error"].as_str())
        .unwrap_or("unknown error")
        .to_string()
}

fn pick_image(images: &Value, target: u32) -> String {
    let mut best = String::new();
    let mut best_diff = u32::MAX;

    for image in images.as_array().into_iter().flatten() {
        let Some(url) = image["url"].as_str().filter(|url| !url.is_empty()) else {
            continue;
        };
        let width = image["width"].as_u64().unwrap_or(0) as u32;
        let diff = width.abs_diff(target);
        if diff < best_diff {
            best_diff = diff;
            best = url.to_string();
        }
    }

    best
}

fn join_artists(artists: &Value) -> (String, String) {
    let names = artists
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|artist| artist["name"].as_str())
        .collect::<Vec<_>>();

    let first_id = artists
        .as_array()
        .and_then(|list| list.first())
        .and_then(|artist| artist["id"].as_str())
        .unwrap_or_default()
        .to_string();

    (names.join(", "), first_id)
}

fn track_from_value(track: &Value, album_override: Option<&Value>) -> Option<SpotifyTrack> {
    let id = track["id"].as_str()?;
    let album = album_override.unwrap_or_else(|| &track["album"]);
    let images = &album["images"];
    let (artist, artist_id) = join_artists(&track["artists"]);

    Some(SpotifyTrack {
        id: track_key(id),
        title: track["name"].as_str().unwrap_or("Unknown").to_string(),
        artist: if artist.is_empty() {
            "Unknown".to_string()
        } else {
            artist
        },
        artist_id,
        album: album["name"].as_str().unwrap_or("Unknown").to_string(),
        album_id: album["id"].as_str().unwrap_or_default().to_string(),
        duration: track["duration_ms"].as_u64().unwrap_or(0) / 1000,
        cover_small: pick_image(images, 300),
        cover_medium: pick_image(images, 640),
        preview: track["preview_url"].as_str().map(str::to_string),
        track_number: track["track_number"].as_u64().unwrap_or(0) as u32,
        disc_number: track["disc_number"].as_u64().unwrap_or(1) as u32,
        release_date: album["release_date"].as_str().map(str::to_string),
        isrc: track["external_ids"]["isrc"].as_str().map(str::to_string),
        provider: "spotify".to_string(),
    })
}

fn album_from_value(album: &Value) -> Option<SpotifyAlbum> {
    let id = album["id"].as_str()?;
    let images = &album["images"];
    let (artist, artist_id) = join_artists(&album["artists"]);

    Some(SpotifyAlbum {
        id: id.to_string(),
        title: album["name"].as_str().unwrap_or("Unknown").to_string(),
        artist: if artist.is_empty() {
            "Unknown".to_string()
        } else {
            artist
        },
        artist_id,
        cover_small: pick_image(images, 300),
        cover_medium: pick_image(images, 640),
        nb_tracks: album["total_tracks"].as_u64().unwrap_or(0),
        release_date: album["release_date"].as_str().map(str::to_string),
        provider: "spotify".to_string(),
    })
}

fn artist_from_value(artist: &Value) -> Option<SpotifyArtist> {
    let id = artist["id"].as_str()?;
    let images = &artist["images"];

    Some(SpotifyArtist {
        id: id.to_string(),
        name: artist["name"].as_str().unwrap_or("Unknown").to_string(),
        picture_small: pick_image(images, 300),
        picture_medium: pick_image(images, 640),
        nb_album: 0,
        nb_fan: artist["followers"]["total"].as_u64().unwrap_or(0),
        provider: "spotify".to_string(),
    })
}

fn playlist_from_value(playlist: &Value) -> Option<SpotifyPlaylist> {
    let id = playlist["id"].as_str()?;
    let images = &playlist["images"];

    Some(SpotifyPlaylist {
        id: id.to_string(),
        title: playlist["name"].as_str().unwrap_or("Unknown").to_string(),
        creator: playlist["owner"]["display_name"]
            .as_str()
            .or_else(|| playlist["owner"]["id"].as_str())
            .unwrap_or("Spotify")
            .to_string(),
        cover_small: pick_image(images, 300),
        cover_medium: pick_image(images, 640),
        nb_tracks: playlist["tracks"]["total"].as_u64().unwrap_or(0),
        provider: "spotify".to_string(),
    })
}
