use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::Session;
use librespot_oauth::{OAuthClient, OAuthClientBuilder, OAuthToken};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

const KEYRING_SERVICE: &str = crate::migration::IDENTIFIER;
pub(crate) const KEYRING_USER: &str = "spotify_oauth";
pub(crate) const KEYRING_WEB_USER: &str = "spotify_oauth_web";

/// librespot's own keymaster client. Its registered redirect is the loopback
/// URI below, so no Spotify developer app is required. The AP session (track
/// metadata and audio) always uses it: Spotify rejects AP metadata requests
/// made under any other client ID with a 400.
const DEFAULT_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const REDIRECT_URI: &str = "http://127.0.0.1:5588/login";
/// The port in `REDIRECT_URI`, where librespot-oauth listens for the callback.
const CALLBACK_PORT: u16 = 5588;

/// How long a browser login may stay open before its callback listener is
/// shut down and the port released.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// librespot-oauth refreshes with a client that has no timeout of its own.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);
/// Access tokens live for an hour; a stored expiry further out than this is
/// corrupt or tampered with and is clamped.
const MAX_TOKEN_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

/// librespot-oauth's error when the callback request carries no `code`
/// (a stray favicon or prefetch request, or our own timeout request).
const AUTH_CODE_MISSING: &str = "Auth code param not found";
const LOGIN_TIMEOUT_MESSAGE: &str =
    "Spotify login timed out after 5 minutes without an answer from the browser. Click Connect to try again.";

// The empty icon stops the browser asking the (about to be reused) callback
// port for /favicon.ico, which a following login would mistake for its answer.
const LOGIN_DONE_HTML: &str = "<html><head><link rel='icon' href='data:,'></head>\
<body style='font-family:sans-serif;padding:2em'>\
<h2>Wavekeep is connected to Spotify</h2><p>You can close this tab.</p></body></html>";

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

/// Web API credential bound to the user's own client ID. Search draws on that
/// app's quota instead of the one shared by every librespot install.
struct WebAuth {
    oauth: OAuthClient,
    client_id: String,
    token: RwLock<OAuthToken>,
}

#[derive(Serialize, Deserialize)]
struct StoredWebToken {
    client_id: String,
    access_token: String,
    refresh_token: String,
    expires_at_unix: u64,
}

pub struct SpotifyClient {
    pub session: Session,
    pub oauth: OAuthClient,
    pub http: reqwest::Client,
    token: RwLock<OAuthToken>,
    web: Option<WebAuth>,
    /// Serialises refreshes: Spotify may rotate refresh tokens, so concurrent
    /// requests must not each spend the same one.
    refresh_gate: tokio::sync::Mutex<()>,
    /// Whether refreshed tokens are still written to the keyring. Held across
    /// each write, so `stop_persisting` waits for one in flight and a client
    /// that was logged out or replaced can never bring its token back.
    persist: tokio::sync::Mutex<bool>,
    pub profile: Profile,
}

/// A refreshed credential waiting to be written to the keyring.
enum PendingSave {
    Default(StoredToken),
    Web(StoredWebToken),
}

impl PendingSave {
    fn save(&self) -> Result<(), String> {
        match self {
            PendingSave::Default(token) => save_token(token),
            PendingSave::Web(token) => save_web_token(token),
        }
    }
}

impl SpotifyClient {
    /// Stops this client from writing refreshed tokens to the keyring. Called
    /// before its credential is cleared (logout) or replaced (a new login).
    pub async fn stop_persisting(&self) {
        *self.persist.lock().await = false;
    }

    /// Returns a valid bearer token, refreshing it shortly before expiry.
    pub async fn access_token(&self) -> Result<String, String> {
        let lock = match &self.web {
            Some(web) => &web.token,
            None => &self.token,
        };
        let still_valid = {
            let token = lock.read().await;
            token.expires_at > Instant::now() + Duration::from_secs(30)
        };
        if still_valid {
            return Ok(lock.read().await.access_token.clone());
        }
        self.refresh_token().await
    }

    /// Forces a refresh, e.g. after the Web API answered 401.
    pub async fn refresh_token(&self) -> Result<String, String> {
        let (oauth, lock) = match &self.web {
            Some(web) => (&web.oauth, &web.token),
            None => (&self.oauth, &self.token),
        };
        let seen_access_token = lock.read().await.access_token.clone();
        let gate = self.refresh_gate.lock().await;
        {
            // Another request refreshed while this one waited for the gate.
            let token = lock.read().await;
            if token.access_token != seen_access_token {
                return Ok(token.access_token.clone());
            }
        }

        let refresh_token = lock.read().await.refresh_token.clone();
        let fresh = refresh_with_timeout(oauth, &refresh_token)
            .await
            .map_err(|e| format!("Spotify token refresh failed: {}", e))?;

        let access_token = fresh.access_token.clone();
        let pending = match &self.web {
            Some(web) => PendingSave::Web(StoredWebToken {
                client_id: web.client_id.clone(),
                access_token: fresh.access_token.clone(),
                refresh_token: fresh.refresh_token.clone(),
                expires_at_unix: instant_to_unix(fresh.expires_at),
            }),
            None => PendingSave::Default(StoredToken {
                access_token: fresh.access_token.clone(),
                refresh_token: fresh.refresh_token.clone(),
                expires_at_unix: instant_to_unix(fresh.expires_at),
                profile: self.profile.clone(),
            }),
        };
        *lock.write().await = fresh;

        // Requests waiting at the gate can use the new token now; the keyring
        // write (which can wait on an unlock prompt) happens outside the gate.
        // The persistence lock is taken before the gate is released, so saves
        // still land in refresh order and the newest refresh token wins.
        let persist = self.persist.lock().await;
        drop(gate);
        // Persistence is best-effort: a missing keyring must not break playback.
        if *persist {
            if let Err(error) = blocking(move || pending.save()).await {
                crate::logging::warn("spotify", format!("Could not persist the refreshed Spotify credential: {}", error));
            }
        }
        Ok(access_token)
    }
}

/// `web_client_id` is the user's own client ID (or the default). When it is
/// not the default, a second login is made for it and used for the Web API.
pub async fn login(device_id: &str, web_client_id: &str) -> Result<Arc<SpotifyClient>, String> {
    let token = browser_login(DEFAULT_CLIENT_ID).await.map_err(|e| match e {
        BrowserLoginError::TimedOut => e.to_string(),
        BrowserLoginError::Failed(e) => format!("Spotify login failed: {}", e),
    })?;
    let oauth = build_oauth(DEFAULT_CLIENT_ID)?;

    let web = if web_client_id == DEFAULT_CLIENT_ID {
        None
    } else {
        // Let the first tab finish loading before the port is reused.
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mut attempts = 0;
        let web_token = loop {
            attempts += 1;
            match browser_login(web_client_id).await {
                Ok(token) => break token,
                // A stray browser request (favicon, prefetch) hit the callback.
                Err(BrowserLoginError::Failed(e)) if attempts < 3 && e.contains(AUTH_CODE_MISSING) => {
                    crate::logging::warn("spotify", format!(
                        "web login: stray callback request, retrying ({})",
                        e
                    ));
                }
                Err(e @ BrowserLoginError::TimedOut) => return Err(e.to_string()),
                Err(BrowserLoginError::Failed(e)) => {
                    return Err(format!("Spotify login with your Client ID failed: {}", e));
                }
            }
        };
        let stored = StoredWebToken {
            client_id: web_client_id.to_string(),
            access_token: web_token.access_token.clone(),
            refresh_token: web_token.refresh_token.clone(),
            expires_at_unix: instant_to_unix(web_token.expires_at),
        };
        if let Err(error) = blocking(move || save_web_token(&stored)).await {
            crate::logging::warn("spotify", format!("Could not persist the custom Client ID credential: {}", error));
        }
        Some(WebAuth {
            oauth: build_oauth(web_client_id)?,
            client_id: web_client_id.to_string(),
            token: RwLock::new(web_token),
        })
    };

    connect(oauth, token, device_id, web).await
}

#[derive(Debug)]
enum BrowserLoginError {
    /// Nothing came back from the browser within `LOGIN_TIMEOUT`.
    TimedOut,
    /// librespot-oauth's error text.
    Failed(String),
}

impl std::fmt::Display for BrowserLoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserLoginError::TimedOut => f.write_str(LOGIN_TIMEOUT_MESSAGE),
            BrowserLoginError::Failed(error) => f.write_str(error),
        }
    }
}

/// Opens the browser for `client_id` and waits for its loopback callback.
async fn browser_login(client_id: &str) -> Result<OAuthToken, BrowserLoginError> {
    let oauth = build_oauth(client_id).map_err(BrowserLoginError::Failed)?;
    let callback = SocketAddr::from(([127, 0, 0, 1], CALLBACK_PORT));
    run_oauth_flow(oauth, callback, LOGIN_TIMEOUT).await
}

/// librespot-oauth's listener uses blocking std sockets with no timeout, so
/// it runs on a blocking thread instead of a runtime worker. When `limit`
/// passes, the listener is woken with a request that carries no code, so it
/// answers, returns, and releases the port for the next attempt.
///
/// librespot-oauth discards the OAuth `state` it generates, so the callback
/// cannot be checked against it; PKCE still keeps a stolen code useless.
async fn run_oauth_flow(
    oauth: OAuthClient,
    callback: SocketAddr,
    limit: Duration,
) -> Result<OAuthToken, BrowserLoginError> {
    let mut flow = tokio::task::spawn_blocking(move || {
        oauth.get_access_token().map_err(|e| e.to_string())
    });

    match tokio::time::timeout(limit, &mut flow).await {
        Ok(joined) => joined
            .map_err(|e| BrowserLoginError::Failed(format!("login task failed: {}", e)))?
            .map_err(BrowserLoginError::Failed),
        Err(_) => {
            crate::logging::warn("spotify", "Browser login timed out; releasing the callback port");
            // A few tries, in case the listener was not yet bound at the first.
            for _ in 0..3 {
                let _ = tokio::task::spawn_blocking(move || release_callback(callback)).await;
                if tokio::time::timeout(Duration::from_secs(1), &mut flow).await.is_ok() {
                    return Err(BrowserLoginError::TimedOut);
                }
            }
            crate::logging::warn(
                "spotify",
                "The login callback listener did not stop; the port may stay busy until restart",
            );
            Err(BrowserLoginError::TimedOut)
        }
    }
}

/// Sends the waiting callback listener a request without a code and waits
/// briefly for its answer, so it returns and closes the port.
fn release_callback(callback: SocketAddr) {
    let Ok(mut stream) = TcpStream::connect_timeout(&callback, Duration::from_secs(1)) else {
        return;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.write_all(
        b"GET /login?error=wavekeep_timeout HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    let mut answer = [0u8; 256];
    let _ = stream.read(&mut answer);
}

/// Recreates the client from the stored refresh token, if there is one.
pub async fn restore(
    device_id: &str,
    web_client_id: &str,
) -> Result<Option<Arc<SpotifyClient>>, String> {
    let Some(stored) = blocking(load_token).await? else {
        return Ok(None);
    };

    let oauth = build_oauth(DEFAULT_CLIENT_ID)?;
    let mut token = OAuthToken {
        access_token: stored.access_token,
        refresh_token: stored.refresh_token,
        expires_at: unix_to_instant(stored.expires_at_unix),
        token_type: "Bearer".to_string(),
        scopes: SCOPES.iter().map(|scope| scope.to_string()).collect(),
    };

    if token.expires_at <= Instant::now() + Duration::from_secs(60) {
        match refresh_with_timeout(&oauth, &token.refresh_token).await {
            Ok(fresh) => token = fresh,
            // No answer says nothing about the token; keep it for the next start.
            Err(RefreshError::TimedOut) => {
                return Err("Spotify did not answer the token refresh in time".to_string());
            }
            Err(RefreshError::Failed(_)) => {
                // A revoked refresh token is useless; force a fresh login.
                let _ = blocking(clear_token).await;
                return Ok(None);
            }
        }
    }

    let profile = if stored.profile.id.is_empty() {
        None
    } else {
        Some(stored.profile)
    };
    let web = restore_web(web_client_id).await;

    // A stored token that the AP rejects (for example one issued to another
    // client ID by an older build) gets one refresh with the default client
    // before giving up, instead of failing on every start.
    let session = match open_session(device_id, &token.access_token).await {
        Ok(session) => session,
        Err(first_error) => {
            crate::logging::warn("spotify", format!(
                "stored token rejected, trying a refresh: {}",
                first_error
            ));
            match refresh_with_timeout(&oauth, &token.refresh_token).await {
                Ok(fresh) => {
                    token = fresh;
                    open_session(device_id, &token.access_token).await?
                }
                Err(RefreshError::TimedOut) => {
                    return Err("Spotify did not answer the token refresh in time".to_string());
                }
                Err(RefreshError::Failed(_)) => {
                    // Not usable with the default client: ask for a fresh login.
                    let _ = blocking(clear_token).await;
                    return Ok(None);
                }
            }
        }
    };
    finish_connect(session, oauth, token, web, profile).await.map(Some)
}

/// Restores the Web API credential for the user's own client ID. Without one
/// (never logged in with it, or the ID changed) the default token is used, which
/// shares its quota with every librespot install.
async fn restore_web(web_client_id: &str) -> Option<WebAuth> {
    if web_client_id == DEFAULT_CLIENT_ID {
        return None;
    }
    let Some(stored) = blocking(load_web_token).await.ok().flatten() else {
        crate::logging::warn("spotify", "no stored credential for the custom Client ID; reconnect to use it");
        return None;
    };
    if stored.client_id != web_client_id {
        crate::logging::warn("spotify", "stored credential belongs to another Client ID; reconnect to use the new one");
        return None;
    }

    let oauth = build_oauth(web_client_id).ok()?;
    let mut token = OAuthToken {
        access_token: stored.access_token,
        refresh_token: stored.refresh_token,
        expires_at: unix_to_instant(stored.expires_at_unix),
        token_type: "Bearer".to_string(),
        scopes: SCOPES.iter().map(|scope| scope.to_string()).collect(),
    };
    if token.expires_at <= Instant::now() + Duration::from_secs(60) {
        token = match refresh_with_timeout(&oauth, &token.refresh_token).await {
            Ok(fresh) => fresh,
            Err(e) => {
                crate::logging::warn("spotify", format!("custom Client ID token refresh failed: {}", e));
                return None;
            }
        };
    }

    Some(WebAuth {
        oauth,
        client_id: web_client_id.to_string(),
        token: RwLock::new(token),
    })
}

/// Removes both stored credentials (on a blocking thread: the keyring can
/// wait on D-Bus or an unlock prompt).
pub async fn logout() -> Result<(), String> {
    blocking(|| {
        let _ = clear_web_token();
        clear_token()
    })
    .await
}

pub fn is_default_client_id(client_id: &str) -> bool {
    client_id == DEFAULT_CLIENT_ID
}

/// Resolves the client ID to use: an explicit setting, the environment
/// override, or librespot's shared keymaster ID.
pub fn configured_client_id(settings_value: &str) -> String {
    let settings_value = settings_value.trim();
    if !settings_value.is_empty() {
        return settings_value.to_string();
    }
    crate::settings::app_env("SPOTIFY_CLIENT_ID")
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
    web: Option<WebAuth>,
) -> Result<Arc<SpotifyClient>, String> {
    connect_with_profile(oauth, token, device_id, web, None).await
}

async fn connect_with_profile(
    oauth: OAuthClient,
    token: OAuthToken,
    device_id: &str,
    web: Option<WebAuth>,
    cached_profile: Option<Profile>,
) -> Result<Arc<SpotifyClient>, String> {
    let session = open_session(device_id, &token.access_token).await?;
    finish_connect(session, oauth, token, web, cached_profile).await
}

/// The AP session always uses the default client ID; a custom ID is only valid
/// for the Web API (see `WebAuth`).
async fn open_session(device_id: &str, access_token: &str) -> Result<Session, String> {
    let config = SessionConfig {
        device_id: device_id.to_string(),
        client_id: DEFAULT_CLIENT_ID.to_string(),
        ..SessionConfig::default()
    };

    let session = Session::new(config, None);
    session
        .connect(Credentials::with_access_token(access_token.to_string()), false)
        .await
        .map_err(|e| format!("Spotify session failed: {}", e))?;
    Ok(session)
}

/// Client for the Web API, Pathfinder and cover art. Audio never goes through
/// it (that is the AP session), so an overall timeout is safe.
fn build_http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("Wavekeep/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

async fn finish_connect(
    session: Session,
    oauth: OAuthClient,
    token: OAuthToken,
    web: Option<WebAuth>,
    cached_profile: Option<Profile>,
) -> Result<Arc<SpotifyClient>, String> {
    let http = build_http()?;

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
    if let Err(error) = blocking(move || save_token(&stored)).await {
        crate::logging::warn("spotify", format!("Could not persist the Spotify credential: {}", error));
    }

    Ok(Arc::new(SpotifyClient {
        session,
        oauth,
        http,
        token: RwLock::new(token),
        web,
        refresh_gate: tokio::sync::Mutex::new(()),
        persist: tokio::sync::Mutex::new(true),
        profile,
    }))
}

#[derive(Debug)]
enum RefreshError {
    TimedOut,
    Failed(String),
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshError::TimedOut => f.write_str("no answer from Spotify in time"),
            RefreshError::Failed(error) => f.write_str(error),
        }
    }
}

/// `refresh_token_async` uses its own HTTP client without timeouts, and runs
/// while `refresh_gate` is held, so a hung refresh must not block every caller.
async fn refresh_with_timeout(oauth: &OAuthClient, refresh_token: &str) -> Result<OAuthToken, RefreshError> {
    match tokio::time::timeout(REFRESH_TIMEOUT, oauth.refresh_token_async(refresh_token)).await {
        Ok(result) => result.map_err(|e| RefreshError::Failed(e.to_string())),
        Err(_) => Err(RefreshError::TimedOut),
    }
}

/// Runs keyring access on a blocking thread: the OS credential store can wait
/// on D-Bus or an unlock prompt, which must not stall a runtime worker.
async fn blocking<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|e| format!("Spotify credential task failed: {}", e))?
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn instant_to_unix(instant: Instant) -> u64 {
    unix_now().saturating_add(instant.saturating_duration_since(Instant::now()).as_secs())
}

/// The stored expiry comes from the keyring, so a corrupt or tampered value
/// (e.g. `u64::MAX`) is clamped instead of overflowing `Instant`. At worst the
/// token is then used until the Web API answers 401 and it is refreshed.
fn unix_to_instant(unix: u64) -> Instant {
    let remaining = Duration::from_secs(unix.saturating_sub(unix_now())).min(MAX_TOKEN_LIFETIME);
    let now = Instant::now();
    now.checked_add(remaining).unwrap_or(now)
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keyring error: {}", e))
}

fn web_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_WEB_USER)
        .map_err(|e| format!("Keyring error: {}", e))
}

fn save_web_token(token: &StoredWebToken) -> Result<(), String> {
    let json = serde_json::to_string(token).map_err(|e| e.to_string())?;
    web_entry()?
        .set_password(&json)
        .map_err(|e| format!("Failed to save Spotify credential: {}", e))
}

fn load_web_token() -> Result<Option<StoredWebToken>, String> {
    match web_entry()?.get_password() {
        Ok(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("Stored Spotify credential is unreadable: {}", e)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read Spotify credential: {}", e)),
    }
}

fn clear_web_token() -> Result<(), String> {
    match web_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to clear Spotify credential: {}", e)),
    }
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

    #[test]
    fn setting_wins_and_is_trimmed() {
        assert_eq!(configured_client_id("  my-client  "), "my-client");
    }

    // One test owns the environment variable so parallel tests never race on it.
    #[test]
    fn env_override_then_default_when_setting_is_blank() {
        std::env::set_var("WAVEKEEP_SPOTIFY_CLIENT_ID", " env-client ");
        assert_eq!(configured_client_id(""), "env-client");
        assert_eq!(configured_client_id("   "), "env-client");
        assert_eq!(configured_client_id("setting"), "setting");

        std::env::set_var("WAVEKEEP_SPOTIFY_CLIENT_ID", "  ");
        assert_eq!(configured_client_id(""), DEFAULT_CLIENT_ID);

        // The pre-rename variable still works when the new one is unset.
        std::env::remove_var("WAVEKEEP_SPOTIFY_CLIENT_ID");
        std::env::set_var("DEEZY_SPOTIFY_CLIENT_ID", "old-client");
        assert_eq!(configured_client_id(""), "old-client");

        std::env::remove_var("DEEZY_SPOTIFY_CLIENT_ID");
        assert_eq!(configured_client_id(""), DEFAULT_CLIENT_ID);
    }

    #[test]
    fn callback_port_matches_the_redirect_uri() {
        let url = url::Url::parse(REDIRECT_URI).unwrap();
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        assert_eq!(url.port(), Some(CALLBACK_PORT));
    }

    #[test]
    fn tampered_expiry_is_clamped_instead_of_overflowing() {
        let far = unix_to_instant(u64::MAX);
        let limit = Instant::now() + MAX_TOKEN_LIFETIME;
        assert!(far <= limit);
        assert!(far > Instant::now() + MAX_TOKEN_LIFETIME - Duration::from_secs(60));

        // An expiry in the past means "expired now".
        assert!(unix_to_instant(0) <= Instant::now());

        // A normal hour-long token survives the round trip.
        let hour = unix_now() + 3600;
        let restored = unix_to_instant(hour);
        let remaining = restored.saturating_duration_since(Instant::now()).as_secs();
        assert!((3590..=3600).contains(&remaining), "remaining {remaining}");
        assert!(instant_to_unix(restored).abs_diff(hour) <= 1);
    }

    /// A free loopback port; nothing outside the machine is contacted.
    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    /// The real librespot-oauth flow on a test port, without opening a
    /// browser. Only the authorize URL is printed; no request leaves the host.
    fn test_oauth(port: u16) -> OAuthClient {
        OAuthClientBuilder::new(
            "test-client",
            &format!("http://127.0.0.1:{}/login", port),
            vec!["streaming"],
        )
        .build()
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn abandoned_login_times_out_and_releases_the_port() {
        let port = free_port();
        let callback = SocketAddr::from(([127, 0, 0, 1], port));

        let result = run_oauth_flow(test_oauth(port), callback, Duration::from_millis(1500)).await;
        assert!(matches!(result, Err(BrowserLoginError::TimedOut)), "{result:?}");
        assert!(LOGIN_TIMEOUT_MESSAGE.contains("timed out"));

        // The listener has returned, so the next login can bind the port.
        std::net::TcpListener::bind(callback).expect("callback port is free again");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stray_callback_request_reports_the_missing_code() {
        let port = free_port();
        let callback = SocketAddr::from(([127, 0, 0, 1], port));
        let flow = tokio::spawn(run_oauth_flow(test_oauth(port), callback, Duration::from_secs(20)));

        // Like a favicon request: the listener answers it and gives up, which
        // `login` retries on.
        let stray = tokio::task::spawn_blocking(move || {
            for _ in 0..50 {
                if let Ok(mut stream) = TcpStream::connect(callback) {
                    stream.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
                    let mut answer = String::new();
                    let _ = stream.read_to_string(&mut answer);
                    return answer;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            String::new()
        });

        let answer = stray.await.unwrap();
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer:?}");
        match flow.await.unwrap() {
            Err(BrowserLoginError::Failed(error)) => assert!(error.contains(AUTH_CODE_MISSING), "{error}"),
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
