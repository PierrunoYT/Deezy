use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::Session;
use librespot_oauth::{OAuthClient, OAuthClientBuilder, OAuthToken};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

const KEYRING_SERVICE: &str = "com.pierr.deezy";
const KEYRING_USER: &str = "spotify_oauth";

/// librespot's own keymaster client. Its registered redirect is the loopback
/// URI below, so no Spotify developer app is required.
const DEFAULT_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const REDIRECT_URI: &str = "http://127.0.0.1:5588/login";

const LOGIN_DONE_HTML: &str = "<html><body style='font-family:sans-serif;padding:2em'>\
<h2>Deezy is connected to Spotify</h2><p>You can close this tab.</p></body></html>";

const SCOPES: &[&str] = &[
    "streaming",
    "user-read-private",
    "user-read-email",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-library-read",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub product: String,
    pub country: Option<String>,
    pub image: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    refresh_token: String,
    /// Unix epoch seconds; converted back to an `Instant` on load.
    expires_at_unix: u64,
    profile: Profile,
}

pub struct SpotifyClient {
    pub session: Session,
    pub oauth: OAuthClient,
    pub http: reqwest::Client,
    token: RwLock<OAuthToken>,
    pub profile: Profile,
}

impl SpotifyClient {
    /// Returns a valid bearer token, refreshing it shortly before expiry.
    pub async fn access_token(&self) -> Result<String, String> {
        let still_valid = {
            let token = self.token.read().await;
            token.expires_at > Instant::now() + Duration::from_secs(30)
        };
        if still_valid {
            return Ok(self.token.read().await.access_token.clone());
        }
        self.refresh_token().await
    }

    /// Forces a refresh, e.g. after the Web API answered 401.
    pub async fn refresh_token(&self) -> Result<String, String> {
        let refresh_token = self.token.read().await.refresh_token.clone();
        let fresh = self
            .oauth
            .refresh_token_async(&refresh_token)
            .await
            .map_err(|e| format!("Spotify token refresh failed: {}", e))?;

        let access_token = fresh.access_token.clone();
        let stored = StoredToken {
            access_token: fresh.access_token.clone(),
            refresh_token: fresh.refresh_token.clone(),
            expires_at_unix: instant_to_unix(fresh.expires_at),
            profile: self.profile.clone(),
        };
        *self.token.write().await = fresh;
        // Persistence is best-effort: a missing keyring must not break playback.
        let _ = save_token(&stored);
        Ok(access_token)
    }
}

pub async fn login(device_id: &str, client_id: &str) -> Result<Arc<SpotifyClient>, String> {
    let oauth = build_oauth(client_id)?;
    let token = oauth
        .get_access_token_async()
        .await
        .map_err(|e| format!("Spotify login failed: {}", e))?;
    connect(oauth, token, device_id, client_id).await
}

/// Recreates the client from the stored refresh token, if there is one.
pub async fn restore(
    device_id: &str,
    client_id: &str,
) -> Result<Option<Arc<SpotifyClient>>, String> {
    let Some(stored) = load_token()? else {
        return Ok(None);
    };

    let oauth = build_oauth(client_id)?;
    let mut token = OAuthToken {
        access_token: stored.access_token,
        refresh_token: stored.refresh_token,
        expires_at: unix_to_instant(stored.expires_at_unix),
        token_type: "Bearer".to_string(),
        scopes: SCOPES.iter().map(|scope| scope.to_string()).collect(),
    };

    if token.expires_at <= Instant::now() + Duration::from_secs(60) {
        match oauth.refresh_token_async(&token.refresh_token).await {
            Ok(fresh) => token = fresh,
            Err(_) => {
                // A revoked refresh token is useless; force a fresh login.
                let _ = clear_token();
                return Ok(None);
            }
        }
    }

    let profile = if stored.profile.id.is_empty() {
        None
    } else {
        Some(stored.profile)
    };
    connect_with_profile(oauth, token, device_id, client_id, profile).await.map(Some)
}

pub fn logout() -> Result<(), String> {
    clear_token()
}

/// Resolves the client ID to use: the environment override, or librespot's
/// shared keymaster ID.
pub fn configured_client_id() -> String {
    std::env::var("DEEZY_SPOTIFY_CLIENT_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string())
}

fn build_oauth(client_id: &str) -> Result<OAuthClient, String> {
    OAuthClientBuilder::new(client_id, REDIRECT_URI, SCOPES.to_vec())
        .open_in_browser()
        .with_custom_message(LOGIN_DONE_HTML)
        .build()
        .map_err(|e| format!("Spotify OAuth setup failed: {}", e))
}

async fn connect(
    oauth: OAuthClient,
    token: OAuthToken,
    device_id: &str,
    client_id: &str,
) -> Result<Arc<SpotifyClient>, String> {
    connect_with_profile(oauth, token, device_id, client_id, None).await
}

async fn connect_with_profile(
    oauth: OAuthClient,
    token: OAuthToken,
    device_id: &str,
    client_id: &str,
    cached_profile: Option<Profile>,
) -> Result<Arc<SpotifyClient>, String> {
    let http = reqwest::Client::builder()
        .user_agent(concat!("Deezy/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    // The AP session and the Web API share the same access token.
    let mut config = SessionConfig::default();
    config.device_id = device_id.to_string();
    config.client_id = client_id.to_string();

    let session = Session::new(config, None);
    session
        .connect(Credentials::with_access_token(token.access_token.clone()), false)
        .await
        .map_err(|e| {
            if client_id == DEFAULT_CLIENT_ID {
                format!("Spotify session failed: {}", e)
            } else {
                // Spotify may refuse AP logins for third-party client IDs.
                format!(
                    "Spotify session failed with a custom Client ID: {}. \
                     Unset DEEZY_SPOTIFY_CLIENT_ID to use the default.",
                    e
                )
            }
        })?;

    // The AP already knows the account type and country, so a Web API call is
    // only needed for a display name we may have cached earlier.
    let profile = cached_profile.unwrap_or_else(|| Profile {
        id: session.username(),
        name: session
            .get_user_attribute("name")
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| session.username()),
        product: session
            .get_user_attribute("type")
            .unwrap_or_else(|| "free".to_string()),
        country: Some(session.country()),
        image: None,
    });

    let stored = StoredToken {
        access_token: token.access_token.clone(),
        refresh_token: token.refresh_token.clone(),
        expires_at_unix: instant_to_unix(token.expires_at),
        profile: profile.clone(),
    };
    // Persistence is best-effort: without a keyring the session still works
    // for this run, and the next start asks for a browser login again.
    if let Err(error) = save_token(&stored) {
        eprintln!("Warning: could not persist the Spotify credential: {}", error);
    }

    Ok(Arc::new(SpotifyClient {
        session,
        oauth,
        http,
        token: RwLock::new(token),
        profile,
    }))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn instant_to_unix(instant: Instant) -> u64 {
    unix_now() + instant.saturating_duration_since(Instant::now()).as_secs()
}

fn unix_to_instant(unix: u64) -> Instant {
    let remaining = unix.saturating_sub(unix_now());
    Instant::now() + Duration::from_secs(remaining)
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keyring error: {}", e))
}

fn save_token(token: &StoredToken) -> Result<(), String> {
    let json = serde_json::to_string(token).map_err(|e| e.to_string())?;
    entry()?
        .set_password(&json)
        .map_err(|e| format!("Failed to save Spotify credential: {}", e))
}

fn load_token() -> Result<Option<StoredToken>, String> {
    match entry()?.get_password() {
        Ok(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("Stored Spotify credential is unreadable: {}", e)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read Spotify credential: {}", e)),
    }
}

fn clear_token() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to remove Spotify credential: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test owns the environment variable so parallel tests never race on it.
    #[test]
    fn env_override_then_default_when_blank() {
        std::env::set_var("DEEZY_SPOTIFY_CLIENT_ID", " env-client ");
        assert_eq!(configured_client_id(), "env-client");

        std::env::set_var("DEEZY_SPOTIFY_CLIENT_ID", "  ");
        assert_eq!(configured_client_id(), DEFAULT_CLIENT_ID);

        std::env::remove_var("DEEZY_SPOTIFY_CLIENT_ID");
        assert_eq!(configured_client_id(), DEFAULT_CLIENT_ID);
    }
}
