use super::*;

/// The private API returns every entry of a list when `nb` is -1.
const ALL_ENTRIES: i64 = -1;
const MAX_PROFILE_PLAYLISTS: u32 = 10_000;

fn gw_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn gw_image_url(kind: &str, hash: &str, size: u32) -> String {
    if hash.is_empty() {
        return String::new();
    }
    format!(
        "https://e-cdns-images.dzcdn.net/images/{}/{}/{}x{}-000000-80-0-0.jpg",
        kind, hash, size, size
    )
}

/// Maps a private API song to the shape the public search API returns.
/// User-uploaded files have negative IDs and cannot be downloaded, so they are dropped.
fn gw_track(track: &Value) -> Option<SearchResult> {
    let title = track["SNG_TITLE"].as_str()?;
    // The public API appends the version ("Remastered", "Live") to the title.
    let title = match track["VERSION"].as_str().map(str::trim) {
        Some(version) if !version.is_empty() => format!("{} {}", title, version),
        _ => title.to_string(),
    };
    let picture = track["ALB_PICTURE"].as_str().unwrap_or("");
    let preview = track["MEDIA"].as_array().and_then(|media| {
        media
            .iter()
            .find(|m| m["TYPE"].as_str() == Some("preview"))
            .and_then(|m| m["HREF"].as_str())
            .map(str::to_string)
    });

    Some(SearchResult {
        id: gw_u64(&track["SNG_ID"])?,
        title,
        artist: track["ART_NAME"].as_str().unwrap_or("Unknown").to_string(),
        artist_id: gw_u64(&track["ART_ID"]).unwrap_or(0),
        album: track["ALB_TITLE"].as_str().unwrap_or("Unknown").to_string(),
        duration: gw_u64(&track["DURATION"]).unwrap_or(0),
        cover_small: gw_image_url("cover", picture, 56),
        cover_medium: gw_image_url("cover", picture, 250),
        preview,
    })
}

fn gw_playlist(playlist: &Value, owner_name: &str) -> Option<PlaylistResult> {
    let picture = playlist["PLAYLIST_PICTURE"].as_str().unwrap_or("");
    let kind = playlist["PICTURE_TYPE"].as_str().filter(|k| !k.is_empty()).unwrap_or("playlist");
    Some(PlaylistResult {
        id: gw_u64(&playlist["PLAYLIST_ID"])?,
        title: playlist["TITLE"].as_str()?.to_string(),
        creator: playlist["PARENT_USERNAME"]
            .as_str()
            .filter(|name| !name.is_empty())
            .unwrap_or(owner_name)
            .to_string(),
        cover_small: gw_image_url(kind, picture, 56),
        cover_medium: gw_image_url(kind, picture, 250),
        nb_tracks: gw_u64(&playlist["NB_SONG"]).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::{gw_playlist, gw_track};
    use serde_json::json;

    #[test]
    fn maps_private_api_songs_to_search_results() {
        let song = json!({
            "SNG_ID": "3135556", "SNG_TITLE": "Harder, Better, Faster, Stronger", "VERSION": "(Live)",
            "ART_ID": "27", "ART_NAME": "Daft Punk", "ALB_TITLE": "Discovery", "ALB_PICTURE": "abc",
            "DURATION": "224",
            "MEDIA": [{"TYPE": "preview", "HREF": "https://cdns-preview-d.dzcdn.net/stream/x.mp3"}]
        });
        let track = gw_track(&song).unwrap();
        assert_eq!(track.id, 3135556);
        assert_eq!(track.title, "Harder, Better, Faster, Stronger (Live)");
        assert_eq!(track.artist_id, 27);
        assert_eq!(track.duration, 224);
        assert_eq!(track.cover_small, "https://e-cdns-images.dzcdn.net/images/cover/abc/56x56-000000-80-0-0.jpg");
        assert_eq!(track.preview.as_deref(), Some("https://cdns-preview-d.dzcdn.net/stream/x.mp3"));
    }

    #[test]
    fn drops_user_uploaded_songs() {
        assert!(gw_track(&json!({"SNG_ID": "-123", "SNG_TITLE": "My upload"})).is_none());
    }

    #[test]
    fn maps_profile_playlists_and_falls_back_to_the_owner_name() {
        let playlist = json!({
            "PLAYLIST_ID": "908622995", "TITLE": "Road trip", "NB_SONG": 42,
            "PLAYLIST_PICTURE": "def", "PICTURE_TYPE": "playlist"
        });
        let result = gw_playlist(&playlist, "Sam").unwrap();
        assert_eq!(result.id, 908622995);
        assert_eq!(result.creator, "Sam");
        assert_eq!(result.nb_tracks, 42);
        assert_eq!(result.cover_medium, "https://e-cdns-images.dzcdn.net/images/playlist/def/250x250-000000-80-0-0.jpg");
        assert_eq!(gw_playlist(&json!({"PLAYLIST_ID": "1", "TITLE": "No cover"}), "").unwrap().cover_small, "");
    }
}

impl DeezerClient {
    /// Tracks of any playlist the account can open, including private ones
    /// the public API refuses to return.
    pub async fn get_private_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<SearchResult>, String> {
        let data = self
            .api_call(
                "playlist.getSongs",
                Some(serde_json::json!({ "PLAYLIST_ID": playlist_id, "nb": ALL_ENTRIES })),
            )
            .await?;
        let songs = data["results"]["data"]
            .as_array()
            .ok_or("Failed to get playlist tracks")?;
        Ok(songs.iter().filter_map(gw_track).collect())
    }

    pub async fn get_loved_tracks(&self) -> Result<Vec<SearchResult>, String> {
        let loved_tracks_id = self
            .user
            .as_ref()
            .and_then(|user| user.loved_tracks_id.clone())
            .ok_or("Loved Tracks are not available for this account")?;
        self.get_private_playlist_tracks(&loved_tracks_id).await
    }

    pub async fn get_user_playlists(&self) -> Result<Vec<PlaylistResult>, String> {
        let user = self.user.as_ref().ok_or("Not logged in. Set your ARL token in Settings.")?;
        let data = self
            .api_call(
                "deezer.pageProfile",
                Some(serde_json::json!({
                    "USER_ID": user.id.to_string(),
                    "tab": "playlists",
                    "nb": MAX_PROFILE_PLAYLISTS,
                })),
            )
            .await?;
        let results = &data["results"];
        let playlists = results["TAB"]["playlists"]["data"]
            .as_array()
            .ok_or("Failed to get your playlists")?;
        let owner_name = results["DATA"]["USER"]["BLOG_NAME"].as_str().unwrap_or(&user.name);
        let loved_tracks_id = user.loved_tracks_id.as_deref().and_then(|id| id.parse::<u64>().ok());

        Ok(playlists
            .iter()
            .filter_map(|playlist| gw_playlist(playlist, owner_name))
            // Loved Tracks has its own tab.
            .filter(|playlist| Some(playlist.id) != loved_tracks_id)
            .collect())
    }
}
