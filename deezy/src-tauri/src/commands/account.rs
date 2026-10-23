use super::*;
use std::time::Duration;

/// Set once the app has committed to exiting; `download_track` refuses new
/// downloads from then on.
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
/// Set when the cleanup in `shut_down_and_exit` starts, so it runs only once.
static EXIT_STARTED: AtomicBool = AtomicBool::new(false);
/// How long exit waits for cancelled downloads to remove their temporary files.
const EXIT_CLEANUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Marks the app as exiting, so no new download starts.
pub fn begin_shutdown() {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
}

pub fn is_shutting_down() -> bool {
    SHUTTING_DOWN.load(Ordering::SeqCst)
}

/// Whether `shut_down_and_exit` has been called.
pub fn exit_started() -> bool {
    EXIT_STARTED.load(Ordering::SeqCst)
}

#[tauri::command]
pub async fn exit_app(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    shut_down_and_exit(state.inner(), &app).await;
    Ok(())
}

/// Cancels every download, waits for them to clean up, then exits. Used by
/// `exit_app` and by the backend's own fallback when the window does not
/// finish closing.
pub async fn shut_down_and_exit(state: &AppState, app: &AppHandle) {
    // Before the map is locked: `download_track` checks the flag while holding
    // the lock, so every download is either cancelled below or never starts.
    begin_shutdown();
    if EXIT_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    {
        let cancellations = state.download_cancellations.lock().await;
        crate::logging::info(
            "app",
            format!("Exit requested; cancelling {} active download(s)", cancellations.len()),
        );
        for flag in cancellations.values() {
            flag.store(true, Ordering::Relaxed);
        }
    }

    // Each download removes its registration only after its temporary files
    // have been cleaned, so wait for that, but not forever: one stuck download
    // (a hung network read, say) must not make the app impossible to close.
    let deadline = tokio::time::Instant::now() + EXIT_CLEANUP_TIMEOUT;
    loop {
        let remaining = state.download_cancellations.lock().await.len();
        if remaining == 0 {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            crate::logging::warn(
                "app",
                format!(
                    "Exiting with {} download(s) still running after {} s",
                    remaining,
                    EXIT_CLEANUP_TIMEOUT.as_secs()
                ),
            );
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    app.exit(0);
}

#[tauri::command]
pub async fn login(
    arl: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Value, String> {
    let client = DeezerClient::new(&arl).await.map_err(|e| {
        crate::logging::warn("deezer", format!("Login failed: {}", e));
        e
    })?;
    crate::logging::info("deezer", format!("Logged in ({})", account_kind(&client)));
    let user = serde_json::to_value(&client.user).map_err(|e| e.to_string())?;

    let _settings_io = state.settings_io.lock().await;
    let mut updated_settings = state.settings.lock().await.clone();
    updated_settings.arl = arl;
    let settings_to_save = updated_settings.clone();
    run_blocking(move || settings_to_save.save(&app)).await?;
    *state.settings.lock().await = updated_settings;

    // Do not expose a client backed by an unpersisted credential.
    *state.client.lock().await = Some(client);

    Ok(user)
}

#[tauri::command]
pub async fn auto_login(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<Option<Value>, String> {
    let settings_io = state.settings_io.lock().await;
    let settings = run_blocking(move || Settings::load(&app)).await?;
    *state.settings.lock().await = settings.clone();
    let _ = grant_folder(&state, &settings.output_dir).await;
    drop(settings_io);
    if settings.arl.trim().is_empty() {
        crate::logging::info("deezer", "No saved ARL; not logging in");
        return Ok(None);
    }

    let client = DeezerClient::new(&settings.arl).await.map_err(|e| {
        crate::logging::warn("deezer", format!("Automatic login failed: {}", e));
        e
    })?;
    crate::logging::info("deezer", format!("Automatic login succeeded ({})", account_kind(&client)));
    let user = serde_json::to_value(&client.user).map_err(|e| e.to_string())?;

    let _settings_io = state.settings_io.lock().await;
    if state.settings.lock().await.arl != settings.arl {
        return Err("Account changed while automatic login was in progress".to_string());
    }
    *state.client.lock().await = Some(client);

    Ok(Some(user))
}

/// The account type for the log; never the user name or any credential.
fn account_kind(client: &DeezerClient) -> &'static str {
    match client.user.as_ref().map(|user| user.is_free_account) {
        Some(true) => "free account",
        Some(false) => "paid account",
        None => "account type unknown",
    }
}
