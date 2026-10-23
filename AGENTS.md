# Wavekeep — Agent Reference

Wavekeep is a Tauri v2 desktop application that lets users download music from Deezer and Spotify. The backend is written in Rust; the frontend is SvelteKit 5 (SPA mode, SSR disabled). All commands run from inside the `deezy/` subdirectory.

---

## Commands

All commands must be run from `deezy/` (not the repo root).

```bash
# Development (starts Vite dev server + Tauri window)
bun run tauri dev
# or: npm run tauri -- dev

# Production build
bun run tauri build

# TypeScript / Svelte type-check (no build artifacts)
bun run check
# or: npm run check
```

```bash
# Frontend tests (node --test, mocked stores and IPC)
npm test

# Rust tests
cargo test --manifest-path src-tauri/Cargo.toml
```

Frontend tests in `tests/*.test.mjs` load modules with hand-written mocks of their imports. When a module gains a new store or Svelte `$store` reference, add it to the mocks or the tests fail.

---

## Repository Layout

```
deezy/                          # All source lives here
  src/                          # SvelteKit frontend
    routes/
      +layout.ts                # sets ssr = false (mandatory for Tauri)
      +layout.svelte             # app init, theme, i18n, download-progress listener
      +page.svelte               # top-level shell: Sidebar + view switcher
    lib/
      components/               # Svelte components (one per view/modal); search/ holds the results pieces
      i18n/                     # svelte-i18n setup + locale JSON files
      audioPlayer.ts            # AudioPlayerManager singleton (preview playback)
      downloadQueue.ts          # DownloadQueueManager singleton (queue + concurrency)
      keyboardShortcuts.ts      # global shortcut registry
      logger.ts                 # frontend logging + console/uncaught error forwarding
      notice.ts                 # startup-warning flag and Info & FAQ item ids
      notifications.ts          # system notification helpers
      rateLimiter.ts            # throttle between download invocations
      stores.ts                 # all Svelte writable stores + shared types
      tray.ts                   # frontend tray manager
      downloadHistory.ts        # download history load/save (debounced) and caps
  tests/                        # frontend tests (node --test, hand-written mocks)
  src-tauri/
    src/
      lib.rs                    # AppState definition + Tauri builder, single-instance, panic hook, window/close-to-tray
      commands.rs               # shared command helpers + module exports
      commands/                 # #[tauri::command] handlers grouped by domain
        account.rs              # login/logout, auto_login, exit_app
        catalog.rs              # Deezer catalog/library IPC, `with_session_retry`
        diagnostics.rs          # write_log, read_recent_log, open_log_folder
        downloads.rs            # download_track / cancel_download (both providers)
        filesystem.rs           # music URL parsing and other filesystem IPC
        history.rs              # download history IPC
        settings.rs             # get/save/update settings (allow-list partial updates)
        spotify.rs              # Spotify auth, search, catalog, and library IPC
        tag_editor.rs           # Tag Editor read/write for MP3, FLAC, OGG, M4A
        themes.rs               # theme IPC (calls ../themes.rs)
        tray.rs                 # tray IPC
      deezer/
        mod.rs                  # DeezerClient struct and wiring; request logic lives in the submodules below
        auth.rs                 # login, getUserData, free-account detection
        catalog.rs              # search, album, artist, playlist requests
        gateway.rs              # private GW API calls
        media.rs                # track data + download URL (media API with legacy fallback)
        download.rs             # streaming download + BF decryption + tagging
        bandwidth.rs            # shared download speed limiter
        library.rs              # user library (Loved Tracks, profile playlists, private playlist tracks) via GW
        crypto.rs               # Blowfish/AES key derivation and URL encryption
        models.rs               # shared Rust structs (Serialize/Deserialize)
      spotify/                  # second download source, native librespot
        mod.rs                  # SpotifyClient re-exports + `spotify:` key helpers
        auth.rs                 # OAuth PKCE login, token refresh, keyring, AP session
        api.rs                  # Web API search/playlists + 401/429 handling
        ap.rs                   # AP catalog/download metadata + bounded track loading
        pathfinder.rs           # track/album/artist search fallback on Web API 429
        download.rs             # librespot AP stream -> .wavekeep.part -> tags
        tags.rs                 # OGG/FLAC/MP3/M4A tagging via lofty
      paths.rs                  # provider-neutral paths + temp-file finalization
      migration.rs              # one-time move of Deezy data and logins to the Wavekeep identifier
      logging.rs                # local diagnostic log (levels, redaction, rotation)
      settings.rs               # Settings struct, load/save, keyring integration
      themes.rs                 # custom theme CRUD
      tray.rs                   # system tray construction and menu updates
    tauri.conf.json             # product name, window size, CSP, bundle config
    Cargo.toml
  package.json
```

---

## Architecture & Data Flow

### Startup sequence
Rust `setup` runs `migration::run` (moves Deezy data and logins, see below) before any IPC is served → `+layout.svelte` mounts → loads download history → loads settings (calls `get_settings`) → initialises i18n → applies theme → calls `auto_login` (reads ARL from OS keyring via Rust) → sets `loggedIn` store → shows the app shell (2.2 s minimum splash) → `+page.svelte` mounts and shows the startup warning unless it was dismissed (see "Startup warning and Info & FAQ"). Spotify restoration runs independently through `spotify_status` and does not block the splash.

### Download pipeline
1. Frontend calls `downloadQueueManager.addToQueue(track)` — queued if `max_concurrent_downloads` (1–10, default 3, mirrored in the `maxConcurrentDownloads` store) are already active. Starts are spaced at least `download_delay_seconds` apart (mirrored in the `downloadDelaySeconds` store); skipped tracks don't count.
2. Queue pops the next item and calls `invoke('download_track', { trackId })`.
3. `download_track` dispatches on the track key. Deezer resolves `song.getData` and a CDN URL; Spotify resolves AP metadata and selects an audio file. With `skip_existing_files`, Deezer checks non-empty `.mp3`/`.flac` files and Spotify checks `.ogg`/`.flac`/`.mp3`/`.m4a`; a match returns `status: "skipped"` before opening an audio stream.
4. Deezer decrypts every **3rd** 2048-byte chunk with Blowfish-CBC; Spotify's AES-CTR stream is decrypted with the track's audio key (`session.audio_key().request`) through `AudioDecrypt`, and Spotify's 167-byte (`0xa7`) header is dropped using a magic-byte check (`audio_start`). Both pace received bytes through the shared `BandwidthLimiter` when `download_speed_limit_kbps` > 0, as one total limit across concurrent downloads.
5. Audio is written to a `.wavekeep.part` temp file, then published to the final path on completion.
6. Rust emits `download-progress` Tauri events with `{ track_id, title, percent, status }` throughout.
7. Deezer writes ID3v2.4 MP3 or Vorbis FLAC tags; Spotify uses `lofty` for OGG/FLAC/MP3/M4A. Cover art is embedded on the temporary file.
8. `+layout.svelte` listens for `download-progress` and updates the `downloads` and `downloadHistory` stores.

### IPC conventions
- All Rust commands return `Result<T, String>`. Errors bubble as rejected `invoke()` promises.
- Multi-word Rust command parameters use camelCase with `#[allow(non_snake_case)]` (e.g. `trackId`, `albumId`).
- The `get_settings` command **strips the ARL** before returning to the renderer — the ARL never crosses the IPC boundary to JavaScript.

### State management
- Global app state lives in `src/lib/stores.ts` as Svelte `writable` stores.
- `DownloadQueueManager` (singleton in `downloadQueue.ts`) is the authoritative source for download concurrency; it also updates the stores.
- Rust holds `AppState` (managed via Tauri's `.manage()`) with independent `Arc<Mutex<Option<DeezerClient>>>` and `Arc<Mutex<Option<Arc<SpotifyClient>>>>` sessions, a `Settings` mutex, and a cancellation map keyed by provider-prefixed track ID.

---

## Key Gotchas

### ARL is stored in the OS credential store, not in settings.json
`Settings::save()` writes settings to disk with `arl: ""`. On first save (or migration from an old build with ARL in the JSON), the ARL is moved to the OS keyring (`keyring` crate). `Settings::load()` then reads it back from keyring. Any code that modifies settings must preserve the ARL through this round-trip — `save_settings` merges a blank `arl` from the frontend with the stored value before saving.

### Session auto-refresh
For Deezer, `download_track` automatically rebuilds `DeezerClient` (re-authenticates) and retries **once** on CSRF or token errors (any error text containing "CSRF" or "token"). The Spotify path has no such retry, and library calls use `with_session_retry` in `commands/catalog.rs`. If the retry also fails, it surfaces "Session expired. Please go to Settings and log in again."

### Free-account quality override
If `user.is_free_account` is true, the effective quality is silently overridden to `MP3_128` regardless of user settings — enforced in both the initial download path and the CSRF-retry path.

### App identifier and the Deezy migration
The app is called **Wavekeep** (formerly Deezy). Its identifier `com.pierr.wavekeep` (`tauri.conf.json`, mirrored by `migration::IDENTIFIER` and checked by a test) names the app-data folder and is the credential-store service for the ARL (`arl_token`) and the Spotify tokens (`spotify_oauth`, `spotify_oauth_web`).
- Up to 0.3.0 the identifier was `com.pierr.deezy`. `migration.rs` runs first in the `setup` hook, before any IPC command can read settings or credentials. It moves `settings.json`, `download_history.json` and `themes/` from the old folder (removing it only when empty) and moves each credential (write, read back, then delete the old one). Existing data in the new location is never overwritten, so it runs on every start and does nothing once done. A new app-data file or credential must be added to its lists, and any future identifier change needs the same kind of migration.
- `bundle.windows.wix.upgradeCode` pins the MSI upgrade code that was derived from the old product name, so MSI installs of Wavekeep upgrade Deezy installs. Never remove or change it.
- The `.exe` (NSIS) installer is keyed on `productName`, so it installs Wavekeep next to Deezy. If Deezy is uninstalled with "Delete the application data" before Wavekeep's first start, the data is gone before it can be moved, so release notes must tell users to start Wavekeep once first.
- WebView storage (`localStorage`, caches) also lives under the identifier on Windows and macOS and is not migrated; the only thing the app keeps there is the startup-notice flag.
- Environment variables are read through `settings::app_env`, which tries `WAVEKEEP_<NAME>` and then the old `DEEZY_<NAME>`.
- The source folder `deezy/`, the GitHub repository `PierrunoYT/Deezy`, the logo files `static/logodeezy.*` and `deezy-mark-mono.svg`, and the `.agents/` scripts still use the old name.

### Version must be bumped in three places simultaneously
`deezy/package.json` → `version`, `deezy/src-tauri/Cargo.toml` → `version`, `deezy/src-tauri/tauri.conf.json` → `version`. All three must match.

### Release versioning rules
Versions follow semantic versioning (`MAJOR.MINOR.PATCH`) as of the next release; earlier releases bumped only the patch number, even for features, so do not copy that.

- **Patch** (`0.3.0` → `0.3.1`): bug fixes, documentation, and internal changes only. No new user-visible feature, setting, source, or screen.
- **Minor** (`0.3.x` → `0.4.0`): any new user-visible feature, setting, or music source, even if it ships with fixes. A release containing a feature is never a patch release.
- **Major** (`1.0.0` and later): breaking changes, such as dropping support for existing settings or files or changing the on-disk format in a way that needs migration. While the version is `0.x`, breaking changes bump the minor number and are called out at the top of the changelog entry.
- Do not bump the version, edit the changelog's version heading, or create a tag unless the user asks for a release. Ordinary commits add entries under `## [Unreleased]` in `CHANGELOG.md` and leave the version files alone.
- When asked to release: choose the number from the rules above and state it with a one-line reason before changing anything. If the user names a number that breaks the rules, say so once and follow their choice.
- The release itself follows the checklist in `DEVELOPMENT.md`: bump all three files, turn the Unreleased entries into a dated `## [X.Y.Z] - YYYY-MM-DD` section (leaving a new empty Unreleased above it), run the tests and checks, then make one commit `chore: prepare version X.Y.Z release` and an annotated tag `vX.Y.Z` with the message `Wavekeep vX.Y.Z`. Do not push the commit or tag unless the user asks.

### Settings and file access rules
- Every settings write takes `AppState.settings_io` first, which serialises writes to `settings.json` and the keyring.
- `update_settings` (`commands/settings.rs`) is an allow-list: a new setting needs a field in `Settings`, validation in `settings.rs`, and an explicit key there; unknown keys return an error.
- The renderer cannot point the backend at arbitrary paths. An `output_dir` change must already be approved (`grant_folder`, via `pick_folder`, `get_settings`, or `auto_login`), and audio or cover-image paths must be granted (`grant_audio_file`, `grant_image_file`) or inside `output_dir`. A new command that takes a path should use the `require_*` helpers in `commands.rs`.

### Svelte 5 runes syntax
The project uses Svelte 5. Components use `$state(...)`, `$props()`, and `$derived(...)` — not Svelte 4 `let`/`$:` reactive declarations. Do not introduce Svelte 4 patterns.

### Custom folder template variables
When `folder_structure` is `Custom`, the template (default: `{artist}/{release_date} - {album}/{track_number} - {title}`) supports: `{artist}`, `{album}`, `{title}`, `{track_number}` (alias `{track}`), `{disc_number}` (alias `{disc}`), `{release_date}`, `{release_year}` (alias `{year}`). Template segments are split on `/` and `\`; each segment is sanitised with `sanitize_path_component`.

### Tag failure is non-fatal
If `write_mp3_tags` / `write_flac_tags` fails, a `tag-writing-error` event is emitted but the download is still marked complete. Tagging runs on the `.wavekeep.part` temp file *before* it is published, so the file is still named `.wavekeep.part` when the event fires and is renamed to its final path right after; the audio is intact either way.

### CSP restricts image and media origins
`tauri.conf.json` CSP allows images only from `*.dzcdn.net`, `api.deezer.com`, `*.scdn.co` and `*.spotifycdn.com`; media from `*.dzcdn.net` and `*.scdn.co`. Adding new external image or media sources requires updating the CSP.

### Deezer uses two separate API surfaces
- **Private GW API** (`https://www.deezer.com/ajax/gw-light.php`) — used for auth (`deezer.getUserData`), track data (`song.getData`), and the user library (`deezer.pageProfile` for playlists, `playlist.getSongs` for Loved Tracks via `USER.LOVEDTRACKS_ID` and private playlists). `get_playlist_tracks` tries the public API first and falls back to `playlist.getSongs`, since the public API refuses private playlists. Requires the ARL cookie and a CSRF token (`checkForm`).
- **Legacy public API** (`https://api.deezer.com`) — used for search, album/artist/playlist metadata. No auth required for search results.

### Spotify is a second, independent download engine
- **Catalog uses Web API + AP + Pathfinder**: track/album/artist search starts on the Web API and falls back to Pathfinder on 429 (currently refused by Spotify, see Rate limits); playlist search and user playlists stay on the Web API. Direct track/album/artist/playlist metadata, Loved Tracks, and download metadata use the librespot AP session; artist discographies fall back to the Web API when AP metadata is rejected. AP track lists use 8 concurrent requests, artist discographies are capped at 120 albums, and total track-metadata failure is an error rather than an empty list.
- **Audio** uses the librespot AP session (`librespot-core` / `-metadata` / `-audio`), which delivers the **encrypted** stream: `AudioFile::open` alone yields unplayable data, so `download.rs` must request the audio key, wrap the stream in `AudioDecrypt`, and skip the Spotify header before writing. Tagging uses `lofty` for OGG/FLAC/MP3/M4A because the existing Deezer `id3` / `metaflac` stack cannot cover all Spotify formats.
- **Quality selection**: `format_order` in `spotify/download.rs` gives each `spotify_quality` setting its own order: `normal` starts at 96 kbps, `high` at 160, `very-high` (default, and any unknown value) at 320, and `lossless` at FLAC 24-bit/FLAC. Each order prefers OGG Vorbis and then MP3 at a bitrate, and falls back to the other bitrates; FLAC is only requested for `lossless`, and AAC/M4A is only the arbitrary last resort when none of those exist.
- **Rate limits**: Web API 429s retry at most twice only when `Retry-After` is at most 5 seconds; longer waits fail immediately. Pathfinder fallback failures are appended to the original rate-limit error. As of 2026-10-23 Spotify refuses Pathfinder for this client (`403 Client/request not allowed` on every operation), so the fallback currently cannot rescue a rate-limited default ID — a custom Client ID is the practical fix.
- **Track keys are prefixed**: Spotify tracks use `spotify:<base62>` in `Track.id`, the queue, history and the cancellation map, so they can never collide with numeric Deezer IDs. `download_track` dispatches on that prefix (`spotify::is_spotify_track`); everything else flows through the unchanged queue.
- **Auth is OAuth PKCE**, not an ARL. `librespot-oauth` opens the browser and listens on `127.0.0.1:5588/login`. Two client IDs are in play: the **AP session** (track metadata + audio) always uses librespot's keymaster default, because Spotify answers AP metadata requests made under any other client ID with `400 Bad Request`; the **Web API** (search and the user's playlist list; Loved Tracks and a playlist's tracks come from the AP) uses the custom ID when one is set (saved `spotify_client_id` setting → `WAVEKEEP_SPOTIFY_CLIENT_ID` → default), so search draws on the user's own quota instead of the one shared by every librespot install. A custom ID therefore needs a second browser login. The Settings screen logs out after the ID changes (`saveSpotifyClientId` calls `spotify_logout`); the backend only trims and saves the value. The default token is stored in the OS keyring under `com.pierr.wavekeep` / `spotify_oauth`, the custom-ID token under `spotify_oauth_web` (with its client ID; ignored if the ID changed), both separate from the ARL; persistence is best-effort. Without a stored custom token (restart before reconnecting) the Web API falls back to the default token.
- **Tagging the temp file**: the `.wavekeep.part` extension hides the format from lofty, so `spotify/tags.rs` sniffs it with `guess_file_type()`. The Tag Editor (`commands/tag_editor.rs`) edits MP3 (`id3`), FLAC (`metaflac`), and OGG/M4A (`lofty`, so Spotify downloads can be edited too); the Downloads view shows its button for `.mp3`, `.flac`, `.ogg` and `.m4a`.
- **Token refresh and restore**: refreshes are serialised through `refresh_gate` (a request that waited re-uses the token the winner just fetched, since Spotify may rotate refresh tokens). `restore` retries once with a refreshed token when the AP rejects the stored one, and clears it (forcing a fresh login) only if that refresh also fails. A download makes one AP metadata request (`ap::track_with_files`) for both catalog metadata and file IDs.
- **Account type is not checked**: `Profile.product` is read from the AP session (`free` when unknown) and shown in the UI, but nothing in the download path gates on it. Whether Spotify serves audio to Free accounts through librespot is unverified for this build; quality is whatever formats the track's AP metadata lists (e.g. OGG 96/160/320 and AAC, no FLAC for many tracks), chosen by `select_file`. Custom-ID Development-mode apps may carry Spotify-side restrictions (user allow-list, owner Premium) that Wavekeep neither checks nor reports.
- **Callback port reuse**: the two logins reuse port 5588, so the "connected" page declares an empty icon (no `/favicon.ico` request), the second login waits 2 s, and it retries on `Auth code param not found` (3 attempts in total) (a stray browser request mistaken for the auth response).
- **Diagnostics**: Spotify download steps, retries, rate limits and Pathfinder rejections are logged; see "Logging (local only)".
- **Settings are Deezer-independent**: `save_settings` no longer requires an ARL, and the Spotify source is usable with no Deezer login. `spotify_status` restores the client on startup.
- **Build pin**: `librespot-core 0.8.0`'s build script resolves `vergen-gitcl 1.0.8` + `vergen 9.1.0`, which pulls two incompatible `vergen-lib` versions and fails to compile. `[build-dependencies] vergen = "=9.0.6"` in `Cargo.toml` keeps it on `vergen-lib 0.1.6`. Do not relax that pin without testing a clean `Cargo.lock`.

### Download URL generation (legacy fallback)
When the media API (`media.deezer.com/v1/get_url`) is unavailable, the URL is constructed by AES-128-ECB encrypting a payload derived from `MD5_ORIGIN`, quality code, `SNG_ID`, and `MEDIA_VERSION`. The padding must align to a 16-byte boundary dynamically — a fixed width of 80 is insufficient for long track IDs.

---

## Theming

Built-in themes are `light`, `dark` and `system`; they only toggle a `light` class on `document.documentElement` and set no variables. Custom themes are JSON files in `<app data dir>/themes/` (managed by the `themes.rs` commands) and are applied as inline CSS variables on `document.documentElement`, which are cleared on every theme change. The active custom theme name is stored in `settings.custom_theme` (the `Theme` type also has a `custom` value). CSS variable names follow kebab-case: `--bg-darkest`, `--accent`, `--text-primary`, etc. See `CSS_VARIABLES` in `+layout.svelte` for the full list.

---

## Logging (local only)

- `src-tauri/src/logging.rs` is the only logger: `logging::{debug, info, warn, error}(area, message)`. Lines go to stderr and to `wavekeep-debug.log` in the system temp folder (under `cargo test` only to stderr, so tests never touch the user's log), formatted `2026-10-23T20:44:32Z INFO  deezer    message`; past 1 MB the file rotates to `wavekeep-debug.old.log`. Do not add `eprintln!` or `println!`.
- The log is **local only**. Nothing uploads it; users copy or open it from Info & FAQ (`read_recent_log`, `open_log_folder` in `commands/diagnostics.rs`) and attach it to a report themselves. Keep it that way: no telemetry and no automatic upload.
- Never log credentials or secrets (ARL, Spotify/OAuth tokens, client tokens), user names, e-mail addresses, or what the user searches for. Log account *type*, track IDs, statuses and error text. As a safety net every line is redacted before writing: the home folder becomes `~`, URL query strings become `?[redacted]`, and unbroken token-like runs of 100+ characters become `[redacted]`.
- Frontend: `src/lib/logger.ts` exports `log(level, area, ...)`, which writes through the `write_log` command (areas show up as `ui-<area>`). `installErrorLogging`, called first in `+layout.svelte`, also copies `console.error`, `console.warn` and uncaught errors into the log, so a plain `console.error` is enough for UI errors.
- Areas in use: `app`, `migration`, `settings`, `deezer`, `spotify`, `download`, `console`/`uncaught`/`ui-*` (frontend). `lib.rs` installs a panic hook that logs the panic with a backtrace.
- Logged today: app start (version, OS), migration steps, Deezer login and automatic login (account type only), free-account quality override, media and legacy URL quality fallbacks, session re-login, every `download_track` call and result, each Spotify download step, Spotify connect/restore/disconnect (account type and default/custom Client ID only), Web API rate limits, Pathfinder rejections, settings-storage warnings, tag-writing failures, panics, and frontend errors.

## Startup warning and Info & FAQ

- `StartupWarning.svelte` is shown by `+page.svelte` when it mounts (that is, after the splash screen) unless the user ticked "Don't show this again". That choice is stored in the webview's `localStorage` under `wavekeep.startupNotice` as the dismissed `NOTICE_VERSION` (`src/lib/notice.ts`); storage may be missing or throw, in which case the warning simply shows every launch. Raise `NOTICE_VERSION` when the warning's meaning changes so it shows again.
- `InfoModal.svelte` (sidebar "Info & FAQ" button) has an "Important" tab and a FAQ made of `<details>` items.
- All text lives under `info.*` in every locale file. The ids are listed in `notice.ts` (`WARNING_ITEMS`, `IMPORTANT_ITEMS`, `FAQ_ITEMS`); an id needs `info.items.<id>.title/.text` or `info.faq.<id>.q/.a` in all six locales (`tests/notice.test.mjs` checks all six). Keep the content consistent with the README and `FAQ.md`.

## Internationalisation

Locale JSON files live in `src/lib/i18n/locales/` (`en`, `fr`, `de`, `es`, `it`, `pt`). The locale is initialised by `initI18n()` in `+layout.svelte` using the `locale` field from settings and kept in sync via the `currentLocale` store. Add new keys to all locale files when adding UI strings.

---

## Adding a New Tauri Command

1. Define the async function in the appropriate `commands/*.rs` module with `#[tauri::command]` (create and export a module from `commands.rs` only when adding a new domain).
2. Register it in the `invoke_handler!(tauri::generate_handler![...])` call in `lib.rs`.
3. Call it from the frontend with `invoke('command_name', { camelCaseParam: value })`.
4. For `AppState` access, add `state: tauri::State<'_, AppState>` as a parameter (order matters: Tauri state params must come before `app: AppHandle` if both are present).
