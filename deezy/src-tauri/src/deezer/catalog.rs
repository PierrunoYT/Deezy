use super::*;

async fn collect_pages<F, Fut>(mut page: Value, mut fetch: F) -> Result<Vec<Value>, String>
where
    F: FnMut(Url) -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    let mut entries = Vec::new();
    let mut visited = std::collections::HashSet::new();
    loop {
        let items = page.get_mut("data").and_then(Value::as_array_mut)
            .ok_or("Invalid catalog page: missing data array")?;
        if entries.len() + items.len() > 100_000 {
            return Err("Catalog exceeds 100,000 entries".to_string());
        }
        entries.append(items);

        let next = match page.get("next") {
            None | Some(Value::Null) => break,
            Some(Value::String(next)) if next.is_empty() => break,
            Some(Value::String(next)) => next,
            _ => return Err("Invalid catalog pagination URL".to_string()),
        };
        let mut url = Url::parse(next).map_err(|_| "Invalid catalog pagination URL")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str() != Some("api.deezer.com")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("Catalog pagination must stay on the Deezer API".to_string());
        }
        // Some legacy API responses use http links. Always fetch over TLS.
        url.set_scheme("https").map_err(|_| "Invalid catalog pagination scheme")?;
        url.set_fragment(None);
        if visited.len() >= 1_000 || !visited.insert(url.to_string()) {
            return Err("Catalog pagination repeated or exceeded its page limit".to_string());
        }
        page = fetch(url).await?;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::collect_pages;
    use serde_json::json;

    #[tokio::test]
    async fn collects_all_pages_in_order_and_upgrades_legacy_links() {
        let first = json!({"data": [{"id": 1}], "next": "http://api.deezer.com/playlist/1/tracks?index=1"});
        let result = collect_pages(first, |url| async move {
            assert_eq!(url.scheme(), "https");
            Ok(json!({"data": [{"id": 2}, {"id": 1}]}))
        }).await.unwrap();
        assert_eq!(result, vec![json!({"id": 1}), json!({"id": 2}), json!({"id": 1})]);
    }

    #[tokio::test]
    async fn rejects_pagination_cycles() {
        let page = json!({"data": [], "next": "https://api.deezer.com/playlist/1/tracks?index=0"});
        assert!(collect_pages(page.clone(), |_| std::future::ready(Ok(page.clone()))).await.is_err());
    }

    #[tokio::test]
    async fn rejects_foreign_pagination_hosts_without_fetching() {
        let page = json!({"data": [], "next": "https://example.com/tracks"});
        assert!(collect_pages(page, |_| async { panic!("must not fetch a foreign host") }).await.is_err());
    }

    #[tokio::test]
    async fn propagates_later_page_failures_instead_of_returning_a_partial_playlist() {
        let page = json!({"data": [{"id": 1}], "next": "https://api.deezer.com/playlist/1/tracks?index=1"});
        assert!(collect_pages(page, |_| async { Err("network failure".to_string()) }).await.is_err());
    }
}

impl DeezerClient {
    async fn collect_catalog_pages(&self, page: Value) -> Result<Vec<Value>, String> {
        collect_pages(page, |url| async move {
            let response = self.http.get(url).send().await
                .map_err(|e| e.without_url().to_string())?;
            response_json(response).await
        }).await
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

        let data: Value = response_json(res)
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

        let data: Value = response_json(res)
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
                let response = self.http
                    .get(&tracks_url)
                    .query(&[("limit", "500")])
                    .send()
                    .await
                    .map_err(|e| format!("Failed to get album tracks: {}", e))?;
                response_json::<Value>(response)
                    .await
                    .map_err(|e| format!("Failed to parse album tracks: {}", e))
            },
            self.get_album(album_id),
        )?;

        let data = tracks_res;
        let album_title = album_data["title"].as_str().unwrap_or("Unknown").to_string();
        let cover_small = album_data["cover_small"].as_str().unwrap_or("").to_string();
        let cover_medium = album_data["cover_medium"].as_str().unwrap_or("").to_string();

        let entries = self.collect_catalog_pages(data).await?;
        let tracks = entries
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

        let data: Value = response_json(res)
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

        let data: Value = response_json(res)
            .await
            .map_err(|e| format!("Failed to parse artist albums: {}", e))?;

        let entries = self.collect_catalog_pages(data).await?;
        let albums = entries
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

        let data: Value = response_json(res)
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

        let data: Value = response_json(res)
            .await
            .map_err(|e| format!("Failed to parse playlist tracks: {}", e))?;

        let cover_small = data["picture_small"].as_str().unwrap_or("").to_string();
        let cover_medium = data["picture_medium"].as_str().unwrap_or("").to_string();

        let entries = self.collect_catalog_pages(data["tracks"].clone()).await?;
        let tracks = entries
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

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to get track: {}", e))?;
        let data: Value = response_json(response)
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
}
