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

    let uris: Vec<SpotifyUri> = context
        .pages
        .iter()
        .flat_map(|page| page.tracks.iter())
        .filter_map(|track| track.uri.as_deref())
        .filter_map(|uri| SpotifyUri::from_uri(uri).ok())
        .filter(|uri| matches!(uri, SpotifyUri::Track { .. }))
        .collect();

    load_tracks(session, uris).await
}
