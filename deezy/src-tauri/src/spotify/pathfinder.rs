//! Search through Spotify's internal Pathfinder (GraphQL) API.
//!
//! The public Web API quota is per client ID and the default one is shared by
//! every librespot install, so it is often exhausted. Pathfinder is what the
//! desktop client searches with and is authenticated by the AP session
//! (login5), so it does not draw on that quota.
//!
//! Queries are persisted: the server only knows them by hash, and a hash goes
//! stale when Spotify ships a new web player. Several hashes are tried in
//! order; when all are stale the caller gets an error and nothing breaks.

use librespot_core::version::spotify_semantic_version;
use librespot_core::SpotifyUri;
use serde_json::{json, Value};

use super::auth::SpotifyClient;
use super::{read_capped, MAX_RESPONSE_BYTES};

const PATHFINDER_URL: &str = "https://api-partner.spotify.com/pathfinder/v2/query";
const APP_PLATFORM: &str = "Win32_x86_64";

/// Operation name plus the persisted-query hashes to try, newest first.
type Operation = (&'static str, &'static [&'static str]);

const TRACKS: Operation = (
    "searchTracks",
    &["b02683192a98dde7966b5e6655a79eeb62713eab703eda9902c932818dd52751"],
);
const ALBUMS: Operation = (
    "searchAlbums",
    &["202cb3305e31e5a0767ba7925f28bd728cf8f8b0217e6da43909056071cd70e9"],
);
const ARTISTS: Operation = (
    "searchArtists",
    &["7bf95d754fdbe32c8b161fbbe54d1ae50974900df4dce4c8f1afcbcad153224d"],
);
/// Searches every type at once; used when a per-type hash is stale.
const DESKTOP: Operation = (
    "searchDesktop",
    &[
        "eef7cc54888d91bdd6802623477873caa3948ae173a0c34fd86827b267e94c03",
        "fcad5a3e0d5af727fb76966f06971c19cfa2275e6ff7671196753e008611873c",
        "3c9d3f60dac5dea3876b6db3f534192b1c1d90032c4233c1bbaba526db41eb31",
        "4801118d4a100f756e833d33984436a3899cff359c532f8fd3aaf174b60b3b49",
    ],
);

const MAX_LIMIT: u32 = 50;

pub async fn search_track_uris(
    client: &SpotifyClient,
    query: &str,
    limit: u32,
) -> Result<Vec<SpotifyUri>, String> {
    search_uris(client, TRACKS, "tracksV2", query, limit, "track").await
}

pub async fn search_album_uris(
    client: &SpotifyClient,
    query: &str,
    limit: u32,
) -> Result<Vec<SpotifyUri>, String> {
    search_uris(client, ALBUMS, "albumsV2", query, limit, "album").await
}

pub async fn search_artist_uris(
    client: &SpotifyClient,
    query: &str,
    limit: u32,
) -> Result<Vec<SpotifyUri>, String> {
    search_uris(client, ARTISTS, "artists", query, limit, "artist").await
}

async fn search_uris(
    client: &SpotifyClient,
    operation: Operation,
    section: &str,
    query: &str,
    limit: u32,
    kind: &str,
) -> Result<Vec<SpotifyUri>, String> {
    let limit = limit.clamp(1, MAX_LIMIT);
    let mut last_error = String::new();

    for (name, hashes) in [operation, DESKTOP] {
        for hash in hashes {
            match query_once(client, name, hash, query, limit).await {
                Ok(body) => return Ok(parse_uris(&body, section, kind, limit as usize)),
                Err(error) => {
                    crate::logging::warn("spotify", format!("Pathfinder {} ({}…): {}", name, &hash[..8], error));
                    last_error = error;
                }
            }
        }
    }

    Err(format!("Spotify search is unavailable right now ({})", last_error))
}

async fn query_once(
    client: &SpotifyClient,
    operation: &str,
    hash: &str,
    query: &str,
    limit: u32,
) -> Result<Value, String> {
    let token = client
        .session
        .login5()
        .auth_token()
        .await
        .map_err(|e| format!("login5 failed: {}", e))?;

    let body = json!({
        "variables": {
            "searchTerm": query,
            "offset": 0,
            "limit": limit,
            "numberOfTopResults": 5,
            "includeAudiobooks": false,
            "includeArtistHasConcertsField": false,
            "includePreReleases": false,
            "includeLocalConcertsField": false,
            "includeAuthors": false,
        },
        "operationName": operation,
        "extensions": { "persistedQuery": { "version": 1, "sha256Hash": hash } },
    });

    let mut request = client
        .http
        .post(PATHFINDER_URL)
        .header("Authorization", format!("{} {}", token.token_type, token.access_token))
        .header("Accept", "application/json")
        .header("app-platform", APP_PLATFORM)
        .header("spotify-app-version", spotify_semantic_version())
        .json(&body);

    // The client token is optional; the request is accepted without it.
    let client_token_error = match client.session.spclient().client_token().await {
        Ok(client_token) => {
            request = request.header("client-token", client_token);
            None
        }
        Err(e) => Some(format!("{:?}", e)),
    };

    let response = request
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    let status = response.status();
    let bytes = read_capped(response, MAX_RESPONSE_BYTES, "Pathfinder response")
        .await
        .map_err(|e| format!("{} (HTTP {})", e, status.as_u16()))?;
    let text = String::from_utf8_lossy(&bytes);
    let value: Value = serde_json::from_str(&text).map_err(|_| {
        let snippet: String = text.chars().take(120).collect();
        format!("{} returned HTTP {}: {}", operation, status.as_u16(), snippet)
    })?;

    // A stale hash answers 412 or a GraphQL `PersistedQueryNotFound` error.
    if !status.is_success() || value.get("errors").is_some() {
        let detail = value["errors"][0]["message"].as_str().unwrap_or("no details");
        // The response body is not logged: it could echo the search query.
        crate::logging::warn("spotify", format!(
            "pathfinder {} HTTP {}: {} client_token_error={:?}",
            operation,
            status.as_u16(),
            detail,
            client_token_error
        ));
        return Err(format!(
            "{} rejected (HTTP {}: {})",
            operation,
            status.as_u16(),
            detail
        ));
    }
    Ok(value)
}

/// Pulls result URIs out of `data.searchV2.<section>.items`, tolerating the
/// two item shapes (`item.data.uri` and `data.uri`) Pathfinder has used.
fn parse_uris(body: &Value, section: &str, kind: &str, limit: usize) -> Vec<SpotifyUri> {
    let prefix = format!("spotify:{}:", kind);
    body["data"]["searchV2"][section]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item["item"]["data"]["uri"]
                .as_str()
                .or_else(|| item["data"]["uri"].as_str())
                .or_else(|| item["uri"].as_str())
        })
        .filter(|uri| uri.starts_with(&prefix))
        .filter_map(|uri| SpotifyUri::from_uri(uri).ok())
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_item_shapes_and_skips_other_kinds() {
        let body = json!({"data": {"searchV2": {"tracksV2": {"items": [
            {"item": {"data": {"uri": "spotify:track:6rqhFgbbKwnb9MLmUQDhG6"}}},
            {"data": {"uri": "spotify:track:4cOdK2wGLETKBW3PvgPWqT"}},
            {"item": {"data": {"uri": "spotify:album:4aawyAB9vmqN3uQ7FjRGTy"}}},
            {"item": {"data": {}}},
        ]}}}});

        let uris = parse_uris(&body, "tracksV2", "track", 10);
        assert_eq!(uris.len(), 2);
    }

    #[test]
    fn respects_the_limit_and_missing_sections() {
        let body = json!({"data": {"searchV2": {"tracksV2": {"items": [
            {"item": {"data": {"uri": "spotify:track:6rqhFgbbKwnb9MLmUQDhG6"}}},
            {"item": {"data": {"uri": "spotify:track:4cOdK2wGLETKBW3PvgPWqT"}}},
        ]}}}});

        assert_eq!(parse_uris(&body, "tracksV2", "track", 1).len(), 1);
        assert!(parse_uris(&body, "albumsV2", "album", 10).is_empty());
        assert!(parse_uris(&json!({}), "tracksV2", "track", 10).is_empty());
    }
}
