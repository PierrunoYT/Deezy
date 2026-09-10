pub mod crypto;
pub mod download;
pub mod models;

use models::{AlbumResult, ArtistResult, PlaylistResult, SearchResult, UserInfo};
use futures::StreamExt;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONNECTION, USER_AGENT};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

const API_URL: &str = "https://www.deezer.com/ajax/gw-light.php";
const LEGACY_API_URL: &str = "https://api.deezer.com";
const MAX_JSON_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct DeezerClient {
    pub http: reqwest::Client,
    pub arl: String,
    pub token: String,
    pub license_token: Option<String>,
    pub user: Option<UserInfo>,
}

mod auth;
mod catalog;
mod gateway;
mod media;


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

fn is_allowed_deezer_url(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }

    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    host == "deezer.com"
        || host.ends_with(".deezer.com")
        || host == "dzcdn.net"
        || host.ends_with(".dzcdn.net")
}

async fn response_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, String> {
    let body = response_bytes(response, MAX_JSON_RESPONSE_BYTES, "Deezer response").await?;
    serde_json::from_slice(&body).map_err(|e| e.to_string())
}

async fn response_bytes(
    response: reqwest::Response,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let response = response.error_for_status().map_err(|e| e.without_url().to_string())?;
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(format!("{} is too large", label));
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.without_url().to_string())?;
        if chunk.len() > max_bytes.saturating_sub(body.len()) {
            return Err(format!("{} is too large", label));
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::{is_allowed_deezer_url, response_bytes};
    use url::Url;

    #[tokio::test]
    async fn rejects_http_error_bodies_before_treating_them_as_media() {
        let response = reqwest::Response::from(
            tauri::http::Response::builder().status(404).body("not an image").unwrap(),
        );
        assert!(response_bytes(response, 1024, "Cover image").await.is_err());
    }

    #[test]
    fn only_allows_https_deezer_hosts() {
        for allowed in [
            "https://www.deezer.com/ajax/gw-light.php",
            "https://api.deezer.com/search",
            "https://e-cdns-proxy-a.dzcdn.net/mobile/1",
        ] {
            assert!(is_allowed_deezer_url(&Url::parse(allowed).unwrap()));
        }

        for rejected in [
            "http://www.deezer.com/",
            "https://deezer.com.evil.example/",
            "https://dzcdn.net.evil.example/",
            "https://127.0.0.1/",
        ] {
            assert!(!is_allowed_deezer_url(&Url::parse(rejected).unwrap()));
        }
    }
}
