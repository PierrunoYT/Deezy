//! Spotify catalog through the AP (`spclient` / Mercury), not the Web API.
//!
//! The Web API quota is per client, and librespot's keymaster client is shared
//! by every install, so it can be globally exhausted. AP requests are made as
//! a normal client and stay well within their own limits.

use futures::{stream, StreamExt};
use librespot_core::{date::Date, Session, SpotifyUri};
use librespot_metadata::artist::Artists;
use librespot_metadata::external_id::ExternalIds;
use librespot_metadata::image::Images;
use librespot_metadata::{Album, Artist, Metadata, Playlist, Track};

use super::api::{SpotifyAlbum, SpotifyArtist, SpotifyTrack};
use super::{track_key, validate_id};

/// Refuse artist discographies that would need an unbounded number of calls.
const MAX_ARTIST_ALBUMS: usize = 120;

/// Metadata requests in flight at once; enough to be quick without hammering
/// the AP.
const METADATA_CONCURRENCY: usize = 8;

fn uri(id: &str) -> Result<SpotifyUri, String> {
    SpotifyUri::from_uri(id).map_err(|e| format!("Invalid Spotify id: {}", e))
}

fn track_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:track:{}", validate_id(id, "track")?))
}

fn album_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:album:{}", validate_id(id, "album")?))
}

fn playlist_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:playlist:{}", validate_id(id, "playlist")?))
}

fn artist_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:artist:{}", validate_id(id, "artist")?))
}

fn id_of(uri: &SpotifyUri) -> String {
    uri.to_id().unwrap_or_default()
}

fn cover_urls(images: &Images) -> (String, String) {
    let mut small = String::new();
    let mut medium = String::new();
    let mut small_diff = u32::MAX;
    let mut medium_diff = u32::MAX;

    for image in images.iter() {
        let Ok(file_id) = image.id.to_base16() else {
            continue;
        };
        let url = format!("https://i.scdn.co/image/{}", file_id);
        // `abs_diff` cannot overflow, even for a nonsense width such as i32::MIN.
        let small_distance = image.width.abs_diff(300);
        if small_distance < small_diff {
            small_diff = small_distance;
            small = url.clone();
        }
        let medium_distance = image.width.abs_diff(640);
        if medium_distance < medium_diff {
            medium_diff = medium_distance;
            medium = url;
        }
    }

    (small, medium)
}

fn release_date(date: &Date) -> Option<String> {
    let year = date.year();
    if year <= 0 {
        return None;
    }
    Some(format!("{:04}-{:02}-{:02}", year, u8::from(date.month()), date.day()))
}

fn isrc(ids: &ExternalIds) -> Option<String> {
    ids.iter()
        .find(|id| id.external_type.eq_ignore_ascii_case("isrc"))
        .map(|id| id.id.clone())
}

fn artist_names(artists: &Artists) -> (String, String) {
    let names = artists
        .iter()
        .map(|artist| artist.name.as_str())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    let first_id = artists.iter().next().map(|artist| id_of(&artist.id)).unwrap_or_default();
    (names.join(", "), first_id)
}

pub fn track_from_metadata(track: &Track) -> Result<SpotifyTrack, String> {
    let id = id_of(&track.id);
    if id.is_empty() {
        return Err("Spotify track has no id".to_string());
    }

    let (artist, artist_id) = artist_names(&track.artists);
    let (cover_small, cover_medium) = cover_urls(&track.album.cover_group);

    Ok(SpotifyTrack {
        id: track_key(&id),
        title: track.name.clone(),
        artist: if artist.is_empty() {
            "Unknown".to_string()
        } else {
            artist
        },
        artist_id,
        album: track.album.name.clone(),
        album_id: id_of(&track.album.id),
        duration: (track.duration.max(0) / 1000) as u64,
        cover_small,
        cover_medium,
        preview: None,
        track_number: track.number.max(0) as u32,
        disc_number: track.disc_number.max(0) as u32,
        release_date: release_date(&track.album.date),
        isrc: isrc(&track.external_ids),
        provider: "spotify".to_string(),
    })
}

pub fn album_from_metadata(album: &Album) -> SpotifyAlbum {
    let (artist, artist_id) = artist_names(&album.artists);
    let (cover_small, cover_medium) = cover_urls(&album.cover_group);

    SpotifyAlbum {
        id: id_of(&album.id),
        title: album.name.clone(),
        artist: if artist.is_empty() {
            "Unknown".to_string()
        } else {
            artist
        },
        artist_id,
        cover_small,
        cover_medium,
        nb_tracks: album.tracks().count() as u64,
        release_date: release_date(&album.date),
        provider: "spotify".to_string(),
    }
}

fn artist_from_metadata(artist: &Artist) -> SpotifyArtist {
    let (picture_small, picture_medium) = cover_urls(&artist.portrait_group);

    SpotifyArtist {
        id: id_of(&artist.id),
        name: artist.name.clone(),
        picture_small,
        picture_medium,
        nb_album: 0,
        nb_fan: artist.popularity.max(0) as u64,
        provider: "spotify".to_string(),
    }
}

pub async fn track(session: &Session, id: &str) -> Result<SpotifyTrack, String> {
    let uri = track_uri(id)?;
    let track = Track::get(session, &uri)
        .await
        .map_err(|e| format!("Spotify track metadata failed: {}", e))?;
    track_from_metadata(&track)
}

/// Like `track`, but also returns the raw AP metadata, whose file IDs a download
/// needs, so the track is fetched from the AP only once.
pub async fn track_with_files(
    session: &Session,
    id: &str,
) -> Result<(SpotifyTrack, Track), String> {
    let uri = track_uri(id)?;
    let raw = Track::get(session, &uri)
        .await
        .map_err(|e| format!("Spotify track metadata failed: {}", e))?;
    let track = track_from_metadata(&raw)?;
    Ok((track, raw))
}

async fn load_track(session: &Session, uri: &SpotifyUri) -> Result<SpotifyTrack, String> {
    let track = Track::get(session, uri)
        .await
        .map_err(|e| format!("Spotify track metadata failed: {}", e))?;
    track_from_metadata(&track)
}

/// Loads metadata for many tracks with a bounded number of AP requests in
/// flight, keeping the original order. Individual failures are skipped
/// (unavailable tracks are normal) but logged; if nothing loads at all, the
/// first error is returned so a broken session is not shown as an empty list.
pub async fn load_tracks(
    session: &Session,
    uris: Vec<SpotifyUri>,
) -> Result<Vec<SpotifyTrack>, String> {
    let results: Vec<Result<SpotifyTrack, String>> = stream::iter(uris)
        .map(|uri| async move { load_track(session, &uri).await })
        .buffered(METADATA_CONCURRENCY)
        .collect()
        .await;

    let mut tracks = Vec::new();
    let mut first_error = None;
    for result in results {
        match result {
            Ok(track) => tracks.push(track),
            Err(error) => {
                crate::logging::warn("spotify", format!("Skipped a track: {}", error));
                first_error.get_or_insert(error);
            }
        }
    }

    match (tracks.is_empty(), first_error) {
        (true, Some(error)) => Err(error),
        _ => Ok(tracks),
    }
}

pub async fn album_tracks(session: &Session, album_id: &str) -> Result<Vec<SpotifyTrack>, String> {
    let uri = album_uri(album_id)?;
    let album = Album::get(session, &uri)
        .await
        .map_err(|e| format!("Spotify album metadata failed: {}", e))?;

    load_tracks(session, album.tracks().cloned().collect()).await
}

pub async fn playlist_tracks(
    session: &Session,
    playlist_id: &str,
) -> Result<Vec<SpotifyTrack>, String> {
    let uri = playlist_uri(playlist_id)?;
    let playlist = Playlist::get(session, &uri)
        .await
        .map_err(|e| format!("Spotify playlist metadata failed: {}", e))?;

    load_tracks(session, playlist.tracks().cloned().collect()).await
}

pub async fn artist_albums(
    session: &Session,
    artist_id: &str,
) -> Result<Vec<SpotifyAlbum>, String> {
    let uri = artist_uri(artist_id)?;
    let artist = Artist::get(session, &uri)
        .await
        .map_err(|e| format!("Spotify artist metadata failed: {}", e))?;

    // Albums, singles and compilations contain several releases of the same
    // album; dedupe by id and keep a stable order.
    let mut seen = std::collections::HashSet::new();
    let mut uris = Vec::new();
    for group in artist
        .albums
        .iter()
        .chain(artist.singles.iter())
        .chain(artist.compilations.iter())
    {
        for album_uri in group.iter() {
            let id = id_of(album_uri);
            if !id.is_empty() && seen.insert(id) && uris.len() < MAX_ARTIST_ALBUMS {
                uris.push(album_uri.clone());
            }
        }
    }

    Ok(albums_from_uris(session, uris).await)
}

/// Album metadata for many URIs, with the same bounded concurrency as tracks.
pub async fn albums_from_uris(session: &Session, uris: Vec<SpotifyUri>) -> Vec<SpotifyAlbum> {
    stream::iter(uris)
        .map(|album_uri| async move {
            Album::get(session, &album_uri)
                .await
                .ok()
                .map(|album| album_from_metadata(&album))
        })
        .buffered(METADATA_CONCURRENCY)
        .filter_map(|album| async move { album })
        .collect()
        .await
}

pub async fn artists_from_uris(session: &Session, uris: Vec<SpotifyUri>) -> Vec<SpotifyArtist> {
    stream::iter(uris)
        .map(|artist_uri| async move {
            Artist::get(session, &artist_uri)
                .await
                .ok()
                .map(|artist| artist_from_metadata(&artist))
        })
        .buffered(METADATA_CONCURRENCY)
        .filter_map(|artist| async move { artist })
        .collect()
        .await
}

/// The user's Liked Songs, exposed by the AP as a collection context.
pub async fn liked_tracks(session: &Session, username: &str) -> Result<Vec<SpotifyTrack>, String> {
    if username.is_empty() {
        return Err("Spotify username is unknown".to_string());
    }
    let uri = format!("spotify:user:{}:collection", username);
    let context = session
        .spclient()
        .get_context(&uri)
        .await
        .map_err(|e| format!("Spotify liked songs failed: {}", e))?;

    let first_pages: Vec<PageInfo> = context
        .pages
        .iter()
        .map(|page| PageInfo {
            tracks: page
                .tracks
                .iter()
                .filter_map(|track| track.uri.clone())
                .collect(),
            page_url: page.page_url.clone(),
            next_page_url: page.next_page_url.clone(),
        })
        .collect();

    let track_uris = collect_context_tracks(first_pages, |url| async move {
        let mut last_error = String::new();
        // One retry: a dropped page would otherwise silently shorten the library.
        for _ in 0..2 {
            match session.spclient().get_next_page(&url).await {
                Ok(bytes) => return pages_from_json(&bytes),
                Err(e) => last_error = e.to_string(),
            }
        }
        Err(format!("Spotify liked songs page failed: {}", last_error))
    })
    .await?;

    let uris: Vec<SpotifyUri> = track_uris
        .iter()
        .filter_map(|uri| SpotifyUri::from_uri(uri).ok())
        .filter(|uri| matches!(uri, SpotifyUri::Track { .. }))
        .collect();

    load_tracks(session, uris).await
}

/// Spotify's own limit for Liked Songs; a safety stop, not a truncation point.
const MAX_CONTEXT_TRACKS: usize = 10_000;

/// Follow-up page requests for one context.
const MAX_CONTEXT_PAGES: usize = 200;

/// One page of a context, as read from the AP.
#[derive(Debug, Default, PartialEq)]
struct PageInfo {
    tracks: Vec<String>,
    page_url: Option<String>,
    next_page_url: Option<String>,
}

fn json_str(value: &serde_json::Value, camel: &str, snake: &str) -> Option<String> {
    value
        .get(camel)
        .or_else(|| value.get(snake))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn page_from_json(value: &serde_json::Value) -> PageInfo {
    PageInfo {
        tracks: value
            .get("tracks")
            .and_then(|t| t.as_array())
            .map(|tracks| {
                tracks
                    .iter()
                    .filter_map(|t| t.get("uri").and_then(|u| u.as_str()))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        page_url: json_str(value, "pageUrl", "page_url"),
        next_page_url: json_str(value, "nextPageUrl", "next_page_url"),
    }
}

/// A follow-up response is either a whole context (`pages`) or a single page.
fn pages_from_json(bytes: &[u8]) -> Result<Vec<PageInfo>, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "Spotify returned an unreadable liked songs page".to_string())?;
    Ok(match value.get("pages").and_then(|p| p.as_array()) {
        Some(pages) => pages.iter().map(page_from_json).collect(),
        None => vec![page_from_json(&value)],
    })
}

/// Only `hm://` paths are requested, so a response cannot steer the session
/// to another scheme or climb out of the path.
fn is_page_url(url: &str) -> bool {
    url.len() <= 2048
        && url.starts_with("hm://")
        && !url[5..].contains("://")
        && !url.contains("..")
        && !url.contains('#')
        && !url.chars().any(|c| c.is_control() || c.is_whitespace() || c == '\\')
}

/// Track URIs of a context in order: the pages already in hand, then every
/// `next_page_url` (and the `page_url` of an empty page) until none is left.
async fn collect_context_tracks<F, Fut>(
    first_pages: Vec<PageInfo>,
    mut fetch: F,
) -> Result<Vec<String>, String>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<PageInfo>, String>>,
{
    let mut tracks: Vec<String> = Vec::new();
    let mut seen_tracks = std::collections::HashSet::new();
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    let mut requests = 0usize;
    let mut pending = first_pages;

    loop {
        for page in pending.drain(..) {
            let empty = page.tracks.is_empty();
            for uri in page.tracks {
                if seen_tracks.insert(uri.clone()) {
                    tracks.push(uri);
                }
            }
            // A page that only carries a link (an artist context) is fetched too.
            if empty {
                if let Some(url) = page.page_url {
                    queue.push_back(url);
                }
            }
            if let Some(url) = page.next_page_url {
                queue.push_back(url);
            }
        }
        if tracks.len() >= MAX_CONTEXT_TRACKS {
            tracks.truncate(MAX_CONTEXT_TRACKS);
            break;
        }
        let Some(url) = queue.pop_front() else { break };
        if !is_page_url(&url) {
            crate::logging::warn("spotify", "Ignored an unexpected liked songs page link");
            continue;
        }
        if !visited.insert(url.clone()) {
            continue;
        }
        if requests >= MAX_CONTEXT_PAGES {
            return Err("Spotify liked songs have too many pages".to_string());
        }
        requests += 1;
        pending = fetch(url).await?;
    }

    crate::logging::info(
        "spotify",
        format!("Liked songs: {} tracks from {} extra pages", tracks.len(), requests),
    );
    Ok(tracks)
}

#[cfg(test)]
mod context_tests {
    use super::*;

    fn page(tracks: &[&str], next: Option<&str>) -> PageInfo {
        PageInfo {
            tracks: tracks.iter().map(|t| t.to_string()).collect(),
            page_url: None,
            next_page_url: next.map(str::to_string),
        }
    }

    #[test]
    fn follows_next_pages_in_order_without_duplicates() {
        let first = vec![page(&["spotify:track:a", "spotify:track:b"], Some("hm://c/p2"))];
        let mut calls = Vec::new();
        let result = futures::executor::block_on(collect_context_tracks(first, |url| {
            calls.push(url.clone());
            async move {
                Ok(match url.as_str() {
                    "hm://c/p2" => vec![page(&["spotify:track:b", "spotify:track:c"], Some("hm://c/p3"))],
                    _ => vec![page(&["spotify:track:d"], None)],
                })
            }
        }))
        .unwrap();
        assert_eq!(result, ["spotify:track:a", "spotify:track:b", "spotify:track:c", "spotify:track:d"]);
        assert_eq!(calls, ["hm://c/p2", "hm://c/p3"]);
    }

    #[test]
    fn stops_on_a_cycle_and_ignores_foreign_links() {
        let first = vec![page(&["spotify:track:a"], Some("https://evil/x")), page(&[], Some("hm://c/p"))];
        let mut calls = 0;
        let result = futures::executor::block_on(collect_context_tracks(first, |_| {
            calls += 1;
            async { Ok(vec![page(&["spotify:track:b"], Some("hm://c/p"))]) }
        }))
        .unwrap();
        assert_eq!(result, ["spotify:track:a", "spotify:track:b"]);
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_failed_page_is_an_error_not_a_short_list() {
        let first = vec![page(&["spotify:track:a"], Some("hm://c/p2"))];
        let result = futures::executor::block_on(collect_context_tracks(first, |_| async {
            Err("boom".to_string())
        }));
        assert_eq!(result, Err("boom".to_string()));
    }

    #[test]
    fn follows_the_link_of_an_empty_page() {
        let mut linked = page(&[], None);
        linked.page_url = Some("hm://c/linked".to_string());
        let result = futures::executor::block_on(collect_context_tracks(vec![linked], |url| async move {
            assert_eq!(url, "hm://c/linked");
            Ok(vec![page(&["spotify:track:z"], None)])
        }))
        .unwrap();
        assert_eq!(result, ["spotify:track:z"]);
    }

    #[test]
    fn reads_a_context_or_a_single_page() {
        let ctx = br#"{"pages":[{"tracks":[{"uri":"spotify:track:a"}],"nextPageUrl":"hm://c/p2"}]}"#;
        let pages = pages_from_json(ctx).unwrap();
        assert_eq!(pages[0].tracks, ["spotify:track:a"]);
        assert_eq!(pages[0].next_page_url.as_deref(), Some("hm://c/p2"));
        let single = br#"{"tracks":[{"uri":"spotify:track:b"}],"next_page_url":"hm://c/p3"}"#;
        let pages = pages_from_json(single).unwrap();
        assert_eq!(pages[0].tracks, ["spotify:track:b"]);
        assert_eq!(pages[0].next_page_url.as_deref(), Some("hm://c/p3"));
        assert!(pages_from_json(b"not json").is_err());
    }

    #[test]
    fn page_urls_must_be_plain_hm_paths() {
        assert!(is_page_url("hm://collection/collection/user/x/tracks?page=2"));
        assert!(!is_page_url("https://example.com/"));
        assert!(!is_page_url("hm://a/../b"));
        assert!(!is_page_url("hm://a b"));
        assert!(!is_page_url("hm://a://b"));
    }
}
