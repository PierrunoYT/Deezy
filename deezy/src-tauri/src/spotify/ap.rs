//! Spotify catalog through the AP (`spclient` / Mercury), not the Web API.
//!
//! The Web API quota is per client, and librespot's keymaster client is shared
//! by every install, so it can be globally exhausted. AP requests are made as
//! a normal client and stay well within their own limits.

use librespot_core::{date::Date, Session, SpotifyUri};
use librespot_metadata::artist::Artists;
use librespot_metadata::external_id::ExternalIds;
use librespot_metadata::image::Images;
use librespot_metadata::{Album, Artist, Metadata, Playlist, Track};

use super::api::{SpotifyAlbum, SpotifyArtist, SpotifyTrack};
use super::track_key;

/// Refuse artist discographies that would need an unbounded number of calls.
const MAX_ARTIST_ALBUMS: usize = 120;

fn uri(id: &str) -> Result<SpotifyUri, String> {
    SpotifyUri::from_uri(id).map_err(|e| format!("Invalid Spotify id: {}", e))
}

fn track_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:track:{}", id))
}

fn album_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:album:{}", id))
}

fn playlist_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:playlist:{}", id))
}

fn artist_uri(id: &str) -> Result<SpotifyUri, String> {
    uri(&format!("spotify:artist:{}", id))
}

fn id_of(uri: &SpotifyUri) -> String {
    uri.to_id().unwrap_or_default()
}

fn cover_urls(images: &Images) -> (String, String) {
    let mut small = String::new();
    let mut medium = String::new();
    let mut small_diff = i32::MAX;
    let mut medium_diff = i32::MAX;

    for image in images.iter() {
        let Ok(file_id) = image.id.to_base16() else {
            continue;
        };
        let url = format!("https://i.scdn.co/image/{}", file_id);
        let small_distance = (image.width - 300).abs();
        if small_distance < small_diff {
            small_diff = small_distance;
            small = url.clone();
        }
        let medium_distance = (image.width - 640).abs();
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

async fn load_track(session: &Session, uri: &SpotifyUri) -> Option<SpotifyTrack> {
    Track::get(session, uri)
        .await
        .ok()
        .and_then(|track| track_from_metadata(&track).ok())
}

pub async fn album_tracks(session: &Session, album_id: &str) -> Result<Vec<SpotifyTrack>, String> {
    let uri = album_uri(album_id)?;
    let album = Album::get(session, &uri)
        .await
        .map_err(|e| format!("Spotify album metadata failed: {}", e))?;

    let mut tracks = Vec::new();
    for track_uri in album.tracks() {
        if let Some(track) = load_track(session, track_uri).await {
            tracks.push(track);
        }
    }
    Ok(tracks)
}

pub async fn playlist_tracks(
    session: &Session,
    playlist_id: &str,
) -> Result<Vec<SpotifyTrack>, String> {
    let uri = playlist_uri(playlist_id)?;
    let playlist = Playlist::get(session, &uri)
        .await
        .map_err(|e| format!("Spotify playlist metadata failed: {}", e))?;

    let mut tracks = Vec::new();
    for track_uri in playlist.tracks() {
        if let Some(track) = load_track(session, track_uri).await {
            tracks.push(track);
        }
    }
    Ok(tracks)
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

    let mut albums = Vec::new();
    for album_uri in uris {
        if let Ok(album) = Album::get(session, &album_uri).await {
            albums.push(album_from_metadata(&album));
        }
    }
    Ok(albums)
}

/// Search is the one operation the AP only exposes as a playable context.
pub async fn search_tracks(
    session: &Session,
    query: &str,
    limit: usize,
) -> Result<Vec<SpotifyTrack>, String> {
    let uri = format!("spotify:search:{}", query.replace(' ', "+"));
    let context = session
        .spclient()
        .get_context(&uri)
        .await
        .map_err(|e| format!("Spotify AP search failed: {}", e))?;

    let uris: Vec<SpotifyUri> = context
        .pages
        .iter()
        .flat_map(|page| page.tracks.iter())
        .filter_map(|track| track.uri.as_deref())
        .filter_map(|uri| SpotifyUri::from_uri(uri).ok())
        .filter(|uri| matches!(uri, SpotifyUri::Track { .. }))
        .take(limit)
        .collect();

    let mut tracks = Vec::new();
    for uri in uris {
        if let Some(track) = load_track(session, &uri).await {
            tracks.push(track);
        }
    }
    Ok(tracks)
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

    let mut tracks = Vec::new();
    for uri in uris {
        if let Some(track) = load_track(session, &uri).await {
            tracks.push(track);
        }
    }
    Ok(tracks)
}
