pub mod crypto;
pub mod download;
pub mod models;

use models::{SearchResult, UserInfo};
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONNECTION, USER_AGENT};
use serde_json::Value;
use std::sync::Arc;

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
        eprintln!("Logging in with ARL: {}...", &self.arl[..20.min(self.arl.len())]);
        
        // Make initial request to establish session and get SID cookie
        let _ = self.http
            .get(API_URL)
            .send()
            .await
            .map_err(|e: reqwest::Error| format!("Failed to get SID: {}", e))?;
        
        eprintln!("Session initialized, getting user data...");
        
        let data = self.api_call("deezer.getUserData", None).await?;
        let results = &data["results"];

        self.token = results["checkForm"]
            .as_str()
            .ok_or("Failed to get auth token from Deezer")?
            .to_string();
        
        eprintln!("Got CSRF token: {} (full: {})", &self.token[..20.min(self.token.len())], self.token);

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
        
        eprintln!("Login successful, user_id: {}", user_id);

        let name = results["USER"]["BLOG_NAME"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        let picture = results["USER"]["USER_PICTURE"]
            .as_str()
            .unwrap_or("");

        let image = if !picture.is_empty() {
            format!(
                "https://e-cdns-images.dzcdn.net/images/user/{}/250x250-000000-80-0-0.jpg",
                picture
            )
        } else {
            "https://e-cdns-images.dzcdn.net/images/user/250x250-000000-80-0-0.jpg".to_string()
        };

        self.user = Some(UserInfo {
            id: user_id,
            name,
            image,
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
                })
            })
            .collect();

        Ok(tracks)
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
            if let Some(result) = self
                .get_media_url(track_token, license_token, quality, fallback)
                .await
            {
                return Ok(result);
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
        let url = crypto::encrypt_download_url(md5_origin, quality_code, &sng_id, media_version);

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

        for q in &["MP3_320", "MP3_128", "FLAC"] {
            if *q == quality {
                continue;
            }
            let qc = get_quality_code(q);
            let url = crypto::encrypt_download_url(md5_origin, qc, &sng_id, media_version);
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
        let url = format!(
            "https://e-cdns-images.dzcdn.net/images/cover/{}/{}x{}.jpg",
            cover_id, size, size
        );
        let res = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        Ok(bytes.to_vec())
    }

    pub async fn get_track_lyrics(&self, track_id: &str) -> Result<Value, String> {
        let params = serde_json::json!({ "SNG_ID": track_id });
        let data = self.api_call("song.getLyrics", Some(params)).await?;
        Ok(data["results"].clone())
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
            if self.token.is_empty() {
                eprintln!("WARNING: Token is empty for method: {}", method);
            }
            self.token.clone()
        };

        let body = params.unwrap_or(serde_json::json!({}));
        
        eprintln!("API call: method={}, token_len={}", method, token.len());

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
                    eprintln!("Deezer API error: {}", msg);
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

pub fn get_quality_ext(quality: &str) -> &str {
    match quality {
        "FLAC" => ".flac",
        "MP3_128" | "MP3_256" | "MP3_320" | "MP4_RA3" => ".mp3",
        "MP4_RA1" | "MP4_RA2" => ".mp4",
        _ => ".mp3",
    }
}

fn extract_string_or_u64(val: &Value) -> Option<String> {
    val.as_str()
        .map(|s| s.to_string())
        .or_else(|| val.as_u64().map(|n| n.to_string()))
        .or_else(|| val.as_i64().map(|n| n.to_string()))
}
