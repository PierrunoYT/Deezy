pub mod crypto;
pub mod download;
pub mod models;

use models::{AlbumResult, ArtistResult, PlaylistResult, SearchResult, UserInfo};
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONNECTION, USER_AGENT};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

const API_URL: &str = "https://www.deezer.com/ajax/gw-light.php";
const LEGACY_API_URL: &str = "https://api.deezer.com";

#[derive(Clone)]
pub struct DeezerClient {
    pub http: reqwest::Client,
    pub arl: String,
    pub token: String,
    pub license_token: Option<String>,
    pub user: Option<UserInfo>,
}

impl DeezerClient {
    pub async fn new(arl: &str) -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=0"));
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));

        let jar = Arc::new(Jar::default());
        let deezer_url = "https://www.deezer.com".parse().unwrap();
        jar.add_cookie_str(&format!("arl={}; Domain=.deezer.com; Path=/", arl), &deezer_url);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_provider(jar)
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let mut client = Self {
            http,
            arl: arl.to_string(),
            token: String::new(),
            license_token: None,
            user: None,
        };

        client.login().await?;
        Ok(client)
    }

    async fn login(&mut self) -> Result<(), String> {
        // Make initial request to establish session and get SID cookie
        let _ = self.http
            .get(API_URL)
            .send()
            .await
            .map_err(|e: reqwest::Error| format!("Failed to get SID: {}", e))?;
        
        let data = self.api_call("deezer.getUserData", None).await?;
        let results = &data["results"];

        self.token = results["checkForm"]
            .as_str()
            .ok_or("Failed to get auth token from Deezer")?
            .to_string();

        self.license_token = results
            .get("USER")
            .and_then(|u| u.get("OPTIONS"))
            .and_then(|o| o.get("license_token"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        let user_id = results["USER"]["USER_ID"]
            .as_u64()
            .or_else(|| {
                results["USER"]["USER_ID"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
            })
            .ok_or("Invalid ARL token")?;

        if user_id == 0 {
            return Err("Invalid ARL token".into());
        }

        let name = results["USER"]["BLOG_NAME"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        let picture = results["USER"]["USER_PICTURE"]
            .as_str()
            .unwrap_or("");

        // Deezer uses a 32-char MD5 hash for USER_PICTURE.
        // An empty or all-zeros hash means the user has no custom picture.
        // Return None so the frontend shows the fallback avatar icon.
        let image = if picture.is_empty() || picture.chars().all(|c| c == '0') {
            None
        } else {
            Some(format!(
                "https://e-cdns-images.dzcdn.net/images/user/{}/250x250-000000-80-0-0.jpg",
                picture
            ))
        };

        let offer_name = results["OFFER_NAME"]
            .as_str()
            .or_else(|| results["USER"]["OFFER_NAME"].as_str())
            .or_else(|| results["USER"]["OPTIONS"]["offer_name"].as_str())
            .unwrap_or("")
            .to_lowercase();
        let has_ads = results["USER"]["OPTIONS"]["ads_audio"].as_bool().unwrap_or(false)
            || results["USER"]["OPTIONS"]["ads_display"].as_bool().unwrap_or(false);
        let is_free_account = offer_name.contains("free")
            || offer_name.contains("gratuit")
            || offer_name.contains("kostenlos")
            || (offer_name.is_empty() && has_ads);

        self.user = Some(UserInfo {
            id: user_id,
            name,
            image,
            is_free_account,
        });

        Ok(())
    }

    pub async fn search_tracks(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SearchResult>, String> {
        let url = format!("{}/search/track", LEGACY_API_URL);

        let res = self
            .http
            .get(&url)
            .query(&[
                ("q", query),
                ("limit", &limit.to_string()),
                ("index", "0"),
            ])
            .send()
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        let data: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse results: {}", e))?;

        if let Some(error) = data.get("error") {
            if let Some(obj) = error.as_object() {
                if !obj.is_empty() {
                    let msg = obj
                        .values()
                        .next()
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Err(format!("API error: {}", msg));
                }
            }
        }

        let tracks = data["data"]
            .as_array()
            .ok_or("No results found")?
            .iter()
            .filter_map(|t| {
                Some(SearchResult {
                    id: t["id"].as_u64()?,
                    title: t["title"].as_str()?.to_string(),
                    artist: t["artist"]["name"].as_str()?.to_string(),
                    artist_id: t["artist"]["id"].as_u64().unwrap_or(0),
                    album: t["album"]["title"].as_str().unwrap_or("Unknown").to_string(),
                    duration: t["duration"].as_u64().unwrap_or(0),
                    cover_small: t["album"]["cover_small"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    cover_medium: t["album"]["cover_medium"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    preview: t["preview"].as_str().map(|s| s.to_string()),
                })
            })
            .collect();

        Ok(tracks)
    }

    pub async fn search_albums(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<AlbumResult>, String> {
        let url = format!("{}/search/album", LEGACY_API_URL);

        let res = self
            .http
            .get(&url)
            .query(&[
                ("q", query),
                ("limit", &limit.to_string()),
                ("index", "0"),
            ])
            .send()
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        let data: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse results: {}", e))?;

        if let Some(error) = data.get("error") {
            if let Some(obj) = error.as_object() {
                if !obj.is_empty() {
                    let msg = obj
                        .values()
                        .next()
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Err(format!("API error: {}", msg));
                }
            }
        }

        let albums = data["data"]
            .as_array()
            .ok_or("No results found")?
            .iter()
            .filter_map(|a| {
                Some(AlbumResult {
                    id: a["id"].as_u64()?,
                    title: a["title"].as_str()?.to_string(),
                    artist: a["artist"]["name"].as_str()?.to_string(),
                    artist_id: a["artist"]["id"].as_u64().unwrap_or(0),
                    cover_small: a["cover_small"].as_str().unwrap_or("").to_string(),
                    cover_medium: a["cover_medium"].as_str().unwrap_or("").to_string(),
                    nb_tracks: a["nb_tracks"].as_u64().unwrap_or(0),
                })
            })
            .collect();

        Ok(albums)
    }

    pub async fn get_album_tracks(
        &self,
        album_id: &str,
    ) -> Result<Vec<SearchResult>, String> {
        let tracks_url = format!("{}/album/{}/tracks", LEGACY_API_URL, album_id);

        // Fetch tracks and album metadata concurrently.
        let (tracks_res, album_data) = tokio::try_join!(
            async {
                self.http
                    .get(&tracks_url)
                    .query(&[("limit", "500")])
                    .send()
                    .await
                    .map_err(|e| format!("Failed to get album tracks: {}", e))?
                    .json::<Value>()
                    .await
                    .map_err(|e| format!("Failed to parse album tracks: {}", e))
            },
            self.get_album(album_id),
        )?;

        let data = tracks_res;
        let album_title = album_data["title"].as_str().unwrap_or("Unknown").to_string();
        let cover_small = album_data["cover_small"].as_str().unwrap_or("").to_string();
        let cover_medium = album_data["cover_medium"].as_str().unwrap_or("").to_string();

        let tracks = data["data"]
            .as_array()
            .ok_or("No tracks found in album")?
            .iter()
            .filter_map(|t| {
                Some(SearchResult {
                    id: t["id"].as_u64()?,
                    title: t["title"].as_str()?.to_string(),
                    artist: t["artist"]["name"].as_str()?.to_string(),
                    artist_id: t["artist"]["id"].as_u64().unwrap_or(0),
                    album: album_title.clone(),
                    duration: t["duration"].as_u64().unwrap_or(0),
                    cover_small: cover_small.clone(),
                    cover_medium: cover_medium.clone(),
                    preview: t["preview"].as_str().map(|s| s.to_string()),
                })
            })
            .collect();

        Ok(tracks)
    }

    pub async fn search_artists(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ArtistResult>, String> {
        let url = format!("{}/search/artist", LEGACY_API_URL);

        let res = self
            .http
            .get(&url)
            .query(&[
                ("q", query),
                ("limit", &limit.to_string()),
                ("index", "0"),
            ])
            .send()
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        let data: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse results: {}", e))?;

        if let Some(error) = data.get("error") {
            if let Some(obj) = error.as_object() {
                if !obj.is_empty() {
                    let msg = obj
                        .values()
                        .next()
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Err(format!("API error: {}", msg));
                }
            }
        }

        let artists = data["data"]
            .as_array()
            .ok_or("No results found")?
            .iter()
            .filter_map(|a| {
                Some(ArtistResult {
                    id: a["id"].as_u64()?,
                    name: a["name"].as_str()?.to_string(),
                    picture_small: a["picture_small"].as_str().unwrap_or("").to_string(),
                    picture_medium: a["picture_medium"].as_str().unwrap_or("").to_string(),
                    nb_album: a["nb_album"].as_u64().unwrap_or(0),
                    nb_fan: a["nb_fan"].as_u64().unwrap_or(0),
                })
            })
            .collect();

        Ok(artists)
    }

    pub async fn get_artist_albums(
        &self,
        artist_id: &str,
    ) -> Result<Vec<AlbumResult>, String> {
        let url = format!("{}/artist/{}/albums", LEGACY_API_URL, artist_id);

        let res = self
            .http
            .get(&url)
            .query(&[("limit", "100")])
            .send()
            .await
            .map_err(|e| format!("Failed to get artist albums: {}", e))?;

        let data: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse artist albums: {}", e))?;

        let albums = data["data"]
            .as_array()
            .ok_or("No albums found for artist")?
            .iter()
            .filter_map(|a| {
                Some(AlbumResult {
                    id: a["id"].as_u64()?,
                    title: a["title"].as_str()?.to_string(),
                    artist: a["artist"]["name"].as_str().unwrap_or("").to_string(),
                    artist_id: a["artist"]["id"].as_u64().unwrap_or(0),
                    cover_small: a["cover_small"].as_str().unwrap_or("").to_string(),
                    cover_medium: a["cover_medium"].as_str().unwrap_or("").to_string(),
                    nb_tracks: a["nb_tracks"].as_u64().unwrap_or(0),
                })
            })
            .collect();

        Ok(albums)
    }

    pub async fn search_playlists(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<PlaylistResult>, String> {
        let url = format!("{}/search/playlist", LEGACY_API_URL);

        let res = self
            .http
            .get(&url)
            .query(&[
                ("q", query),
                ("limit", &limit.to_string()),
                ("index", "0"),
            ])
            .send()
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        let data: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse results: {}", e))?;

        if let Some(error) = data.get("error") {
            if let Some(obj) = error.as_object() {
                if !obj.is_empty() {
                    let msg = obj
                        .values()
                        .next()
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Err(format!("API error: {}", msg));
                }
            }
        }

        let playlists = data["data"]
            .as_array()
            .ok_or("No results found")?
            .iter()
            .filter_map(|p| {
                Some(PlaylistResult {
                    id: p["id"].as_u64()?,
                    title: p["title"].as_str()?.to_string(),
                    creator: p["user"]["name"].as_str().unwrap_or("").to_string(),
                    cover_small: p["picture_small"].as_str().unwrap_or("").to_string(),
                    cover_medium: p["picture_medium"].as_str().unwrap_or("").to_string(),
                    nb_tracks: p["nb_tracks"].as_u64().unwrap_or(0),
                })
            })
            .collect();

        Ok(playlists)
    }

    pub async fn get_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<SearchResult>, String> {
        let url = format!("{}/playlist/{}", LEGACY_API_URL, playlist_id);

        let res = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to get playlist tracks: {}", e))?;

        let data: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse playlist tracks: {}", e))?;

        let cover_small = data["picture_small"].as_str().unwrap_or("").to_string();
        let cover_medium = data["picture_medium"].as_str().unwrap_or("").to_string();

        let tracks = data["tracks"]["data"]
            .as_array()
            .ok_or("No tracks found in playlist")?
            .iter()
            .filter_map(|t| {
                Some(SearchResult {
                    id: t["id"].as_u64()?,
                    title: t["title"].as_str()?.to_string(),
                    artist: t["artist"]["name"].as_str()?.to_string(),
                    artist_id: t["artist"]["id"].as_u64().unwrap_or(0),
                    album: t["album"]["title"].as_str().unwrap_or("Unknown").to_string(),
                    duration: t["duration"].as_u64().unwrap_or(0),
                    cover_small: t["album"]["cover_small"]
                        .as_str()
                        .unwrap_or(&cover_small)
                        .to_string(),
                    cover_medium: t["album"]["cover_medium"]
                        .as_str()
                        .unwrap_or(&cover_medium)
                        .to_string(),
                    preview: t["preview"].as_str().map(|s| s.to_string()),
                })
            })
            .collect();

        Ok(tracks)
    }

    pub async fn get_track_by_id(&self, track_id: &str) -> Result<SearchResult, String> {
        let url = format!("{}/track/{}", LEGACY_API_URL, track_id);

        let data: Value = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to get track: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse track: {}", e))?;

        if let Some(error) = data.get("error") {
            if let Some(message) = error.get("message").and_then(|m| m.as_str()) {
                return Err(format!("API error: {}", message));
            }
        }

        let id = data["id"]
            .as_u64()
            .or_else(|| track_id.parse::<u64>().ok())
            .ok_or("Track not found")?;

        let title = data["title"].as_str().unwrap_or("").to_string();
        if title.is_empty() {
            return Err("Track not found".to_string());
        }

        Ok(SearchResult {
            id,
            title,
            artist: data["artist"]["name"].as_str().unwrap_or("Unknown").to_string(),
            artist_id: data["artist"]["id"].as_u64().unwrap_or(0),
            album: data["album"]["title"].as_str().unwrap_or("Unknown").to_string(),
            duration: data["duration"].as_u64().unwrap_or(0),
            cover_small: data["album"]["cover_small"].as_str().unwrap_or("").to_string(),
            cover_medium: data["album"]["cover_medium"].as_str().unwrap_or("").to_string(),
            preview: data["preview"].as_str().map(|s| s.to_string()),
        })
    }

    pub async fn get_track(&self, track_id: &str) -> Result<Value, String> {
        // Use song.getData method like Python version
        let params = serde_json::json!({ "SNG_ID": track_id });
        let data = self.api_call("song.getData", Some(params)).await?;
        Ok(data["results"].clone())
    }

    pub async fn get_track_download_url(
        &self,
        track: &Value,
        quality: &str,
        fallback: bool,
    ) -> Result<(String, String), String> {
        let track_data = if track.get("DATA").is_some() {
            &track["DATA"]
        } else {
            track
        };

        if let (Some(track_token), Some(ref license_token)) =
            (track_data["TRACK_TOKEN"].as_str(), &self.license_token)
        {
            // Always try requested quality first to avoid unexpectedly
            // receiving lower quality from a multi-format media request.
            if let Some(result) = self
                .get_media_url(track_token, license_token, quality, false)
                .await
            {
                return Ok(result);
            }

            if fallback {
                for fallback_quality in fallback_qualities(quality) {
                    if let Some(result) = self
                        .get_media_url(track_token, license_token, fallback_quality, false)
                        .await
                    {
                        return Ok(result);
                    }
                }
            }
        }

        let md5_origin = track_data["MD5_ORIGIN"]
            .as_str()
            .ok_or("Track unavailable (no MD5_ORIGIN)")?;

        let sng_id = extract_string_or_u64(&track_data["SNG_ID"])
            .ok_or("Track unavailable (no SNG_ID)")?;

        let media_version = track_data["MEDIA_VERSION"]
            .as_str()
            .ok_or("Track unavailable (no MEDIA_VERSION)")?;

        let quality_code = get_quality_code(quality);
        let url = crypto::encrypt_download_url(md5_origin, quality_code, &sng_id, media_version)?;

        let res = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            if let Some(len) = res.content_length() {
                if len > 0 {
                    return Ok((url, quality.to_string()));
                }
            }
        }

        if !fallback {
            return Err("Track not available in requested quality".into());
        }

        for q in fallback_qualities(quality) {
            let qc = get_quality_code(q);
            let url = crypto::encrypt_download_url(md5_origin, qc, &sng_id, media_version)?;
            let res = self
                .http
                .get(&url)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if res.status().is_success() {
                if let Some(len) = res.content_length() {
                    if len > 0 {
                        return Ok((url, q.to_string()));
                    }
                }
            }
        }

        Err("No working download URL found".into())
    }

    pub async fn get_album(&self, album_id: &str) -> Result<Value, String> {
        let url = format!("{}/album/{}", LEGACY_API_URL, album_id);
        let res = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_album_cover(&self, cover_id: &str, size: u32) -> Result<Vec<u8>, String> {
        // Cover images should be well under 10 MiB; cap at 10 MiB to prevent
        // an unexpectedly large response from exhausting memory.
        const MAX_COVER_BYTES: u64 = 10 * 1024 * 1024;

        let url = format!(
            "https://e-cdns-images.dzcdn.net/images/cover/{}/{}x{}.jpg",
            cover_id, size, size
        );
        let res = self.http.get(&url).send().await.map_err(|e| e.to_string())?;

        if let Some(content_length) = res.content_length() {
            if content_length > MAX_COVER_BYTES {
                return Err(format!("Cover image too large: {} bytes", content_length));
            }
        }

        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_COVER_BYTES {
            return Err(format!("Cover image too large: {} bytes", bytes.len()));
        }
        Ok(bytes.to_vec())
    }

    async fn get_media_url(
        &self,
        track_token: &str,
        license_token: &str,
        quality: &str,
        fallback: bool,
    ) -> Option<(String, String)> {
        let mut formats = vec![serde_json::json!({
            "cipher": "BF_CBC_STRIPE",
            "format": quality
        })];

        if fallback {
            for q in &["MP3_320", "MP3_128", "FLAC"] {
                if *q != quality {
                    formats.push(serde_json::json!({
                        "cipher": "BF_CBC_STRIPE",
                        "format": q
                    }));
                }
            }
        }

        let body = serde_json::json!({
            "license_token": license_token,
            "media": [{ "type": "FULL", "formats": formats }],
            "track_tokens": [track_token]
        });

        let res = self
            .http
            .post("https://media.deezer.com/v1/get_url")
            .json(&body)
            .send()
            .await
            .ok()?;

        let result: Value = res.json().await.ok()?;

        let data = result.get("data")?.as_array()?;
        if data.is_empty() {
            return None;
        }

        let media = data[0].get("media")?.as_array()?;
        if media.is_empty() {
            return None;
        }

        let sources = media[0].get("sources")?.as_array()?;
        if sources.is_empty() {
            return None;
        }

        let url = sources[0]["url"].as_str()?.to_string();
        let fmt = media[0]
            .get("format")
            .and_then(|f| f.as_str())
            .unwrap_or(quality)
            .to_string();

        Some((url, fmt))
    }

    pub async fn api_call(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, String> {
        let token = if method == "deezer.getUserData" {
            "null".to_string()
        } else {
            self.token.clone()
        };

        let body = params.unwrap_or(serde_json::json!({}));

        let res = self
            .http
            .post(API_URL)
            .query(&[
                ("api_version", "1.0"),
                ("api_token", &token),
                ("input", "3"),
                ("method", method),
            ])
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("API call failed: {}", e))?;

        let data: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = data.get("error") {
            if let Some(obj) = error.as_object() {
                if !obj.is_empty() {
                    let msg = obj
                        .values()
                        .next()
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Err(format!("Deezer error: {}", msg));
                }
            }
        }

        Ok(data)
    }
}

fn get_quality_code(quality: &str) -> u32 {
    match quality {
        "FLAC" => 9,
        "MP3_128" => 1,
        "MP3_256" => 5,
        "MP3_320" => 3,
        "MP4_RA1" => 13,
        "MP4_RA2" => 14,
        "MP4_RA3" => 15,
        _ => 3,
    }
}

fn fallback_qualities(quality: &str) -> &'static [&'static str] {
    match quality {
        "FLAC" => &["MP3_320", "MP3_128"],
        "MP3_320" => &["MP3_128"],
        _ => &[],
    }
}

pub fn get_quality_ext(quality: &str) -> &str {
    match quality {
        "FLAC" => ".flac",
        "MP3_128" | "MP3_256" | "MP3_320" => ".mp3",
        "MP4_RA1" | "MP4_RA2" | "MP4_RA3" => ".mp4",
        _ => ".mp3",
    }
}

fn extract_string_or_u64(val: &Value) -> Option<String> {
    val.as_str()
        .map(|s| s.to_string())
        .or_else(|| val.as_u64().map(|n| n.to_string()))
        .or_else(|| val.as_i64().map(|n| n.to_string()))
}
