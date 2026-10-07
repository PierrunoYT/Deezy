use super::*;

impl DeezerClient {
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
            .map_err(|e| e.without_url().to_string())?;

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
                .map_err(|e| e.without_url().to_string())?;

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

        response_json(res).await
    }

    pub async fn get_album_cover(&self, cover_id: &str, size: u32) -> Result<Vec<u8>, String> {
        // Cover images should be well under 10 MiB; cap at 10 MiB to prevent
        // an unexpectedly large response from exhausting memory.
        const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;

        let url = format!(
            "https://e-cdns-images.dzcdn.net/images/cover/{}/{}x{}.jpg",
            cover_id, size, size
        );
        let res = self.http.get(&url).send().await.map_err(|e| e.to_string())?;

        response_bytes(res, MAX_COVER_BYTES, "Cover image").await
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

        let result: Value = response_json(res).await.ok()?;

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
        let parsed_url = Url::parse(&url).ok()?;
        if !is_allowed_deezer_url(&parsed_url) {
            return None;
        }
        let fmt = media[0]
            .get("format")
            .and_then(|f| f.as_str())
            .unwrap_or(quality)
            .to_string();

        Some((url, fmt))
    }
}
