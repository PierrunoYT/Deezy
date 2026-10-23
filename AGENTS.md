# Deezy — Agent Reference

Deezy is a Tauri v2 desktop application that lets users download music from Deezer and Spotify. The backend is written in Rust; the frontend is SvelteKit 5 (SPA mode, SSR disabled). All commands run from inside the `deezy/` subdirectory.

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
      components/               # Svelte components (one per view/modal)
      i18n/                     # svelte-i18n setup + locale JSON files
      audioPlayer.ts            # AudioPlayerManager singleton (preview playback)
      downloadQueue.ts          # DownloadQueueManager singleton (queue + concurrency)
      keyboardShortcuts.ts      # global shortcut registry
      notifications.ts          # system notification helpers
      rateLimiter.ts            # throttle between download invocations
      stores.ts                 # all Svelte writable stores + shared types
      tray.ts                   # frontend tray manager
  src-tauri/
    src/
      lib.rs                    # AppState definition + Tauri builder + window setup
      commands.rs               # shared command helpers + module exports
      commands/                 # #[tauri::command] handlers grouped by domain
        spotify.rs              # Spotify auth, search, catalog, and library IPC
      deezer/
        mod.rs                  # DeezerClient (HTTP, login, search, URL resolution)
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
        download.rs             # librespot AP stream -> .deezy.part -> tags
        tags.rs                 # OGG/FLAC/MP3/M4A tagging via lofty
      paths.rs                  # provider-neutral paths + temp-file finalization
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
`+layout.svelte` mounts → loads download history → loads settings (calls `get_settings`) → initialises i18n → applies theme → calls `auto_login` (reads ARL from OS keyring via Rust) → sets `loggedIn` store → shows the app shell (2.2 s minimum splash). Spotify restoration runs independently through `spotify_status` and does not block the splash.

### Download pipeline
1. Frontend calls `downloadQueueManager.addToQueue(track)` — queued if `max_concurrent_downloads` (1–10, default 3, mirrored in the `maxConcurrentDownloads` store) are already active. Starts are spaced at least `download_delay_seconds` apart (mirrored in the `downloadDelaySeconds` store); skipped tracks don't count.
2. Queue pops the next item and calls `invoke('download_track', { trackId })`.
3. `download_track` dispatches on the track key. Deezer resolves `song.getData` and a CDN URL; Spotify resolves AP metadata and selects an audio file. With `skip_existing_files`, Deezer checks non-empty `.mp3`/`.flac` files and Spotify checks `.ogg`/`.flac`/`.mp3`/`.m4a`; a match returns `status: "skipped"` before opening an audio stream.
4. Deezer decrypts every **3rd** 2048-byte chunk with Blowfish-CBC; librespot decrypts Spotify's AES-CTR stream. Both pace received bytes through the shared `BandwidthLimiter` when `download_speed_limit_kbps` > 0, as one total limit across concurrent downloads.
5. Audio is written to a `.deezy.part` temp file, then published to the final path on completion.
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
`download_track` automatically rebuilds `DeezerClient` (re-authenticates) and retries **once** on CSRF or token errors. If the retry also fails, it surfaces "Session expired. Please go to Settings and log in again."

### Free-account quality override
If `user.is_free_account` is true, the effective quality is silently overridden to `MP3_128` regardless of user settings — enforced in both the initial download path and the CSRF-retry path.

### Version must be bumped in three places simultaneously
`deezy/package.json` → `version`, `deezy/src-tauri/Cargo.toml` → `version`, `deezy/src-tauri/tauri.conf.json` → `version`. All three must match.

### Svelte 5 runes syntax
The project uses Svelte 5. Components use `$state(...)`, `$props()`, and `$derived(...)` — not Svelte 4 `let`/`$:` reactive declarations. Do not introduce Svelte 4 patterns.

### Custom folder template variables
When `folder_structure` is `Custom`, the template (default: `{artist}/{release_date} - {album}/{track_number} - {title}`) supports: `{artist}`, `{album}`, `{title}`, `{track_number}` (alias `{track}`), `{disc_number}` (alias `{disc}`), `{release_date}`, `{release_year}` (alias `{year}`). Template segments are split on `/` and `\`; each segment is sanitised with `sanitize_path_component`.

### Tag failure is non-fatal
If `write_mp3_tags` / `write_flac_tags` fails, a `tag-writing-error` event is emitted but the download is still marked complete — the audio file is intact and already renamed from `.deezy.part`.

### CSP restricts image and media origins
`tauri.conf.json` CSP allows images only from `*.dzcdn.net`, `api.deezer.com`, `*.scdn.co` and `*.spotifycdn.com`; media from `*.dzcdn.net` and `*.scdn.co`. Adding new external image or media sources requires updating the CSP.

### Deezer uses two separate API surfaces
- **Private GW API** (`https://www.deezer.com/ajax/gw-light.php`) — used for auth (`deezer.getUserData`), track data (`song.getData`), and the user library (`deezer.pageProfile` for playlists, `playlist.getSongs` for Loved Tracks via `USER.LOVEDTRACKS_ID` and private playlists). `get_playlist_tracks` tries the public API first and falls back to `playlist.getSongs`, since the public API refuses private playlists. Requires the ARL cookie and a CSRF token (`checkForm`).
- **Legacy public API** (`https://api.deezer.com`) — used for search, album/artist/playlist metadata. No auth required for search results.

### Spotify is a second, independent download engine
- **Catalog uses Web API + AP + Pathfinder**: track/album/artist search starts on the Web API and falls back to Pathfinder on 429; playlist search and user playlists stay on the Web API. Direct track/album/artist/playlist metadata, Loved Tracks, and download metadata use the librespot AP session; artist discographies fall back to the Web API when AP metadata is rejected. AP track lists use 8 concurrent requests, artist discographies are capped at 120 albums, and total track-metadata failure is an error rather than an empty list.
- **Audio** uses the librespot AP session (`librespot-core` / `-metadata` / `-audio`), which decrypts the AES-CTR stream. Tagging uses `lofty` for OGG/FLAC/MP3/M4A because the existing Deezer `id3` / `metaflac` stack cannot cover all Spotify formats.
- **Rate limits**: Web API 429s retry at most twice only when `Retry-After` is at most 5 seconds; longer waits fail immediately. Pathfinder fallback failures are appended to the original rate-limit error. As of 2026-10-23 Spotify refuses Pathfinder for this client (`403 Client/request not allowed` on every operation), so the fallback currently cannot rescue a rate-limited default ID — a custom Client ID is the practical fix.
- **Track keys are prefixed**: Spotify tracks use `spotify:<base62>` in `Track.id`, the queue, history and the cancellation map, so they can never collide with numeric Deezer IDs. `download_track` dispatches on that prefix (`spotify::is_spotify_track`); everything else flows through the unchanged queue.
- **Auth is OAuth PKCE**, not an ARL. `librespot-oauth` opens the browser and listens on `127.0.0.1:5588/login`. Two client IDs are in play: the **AP session** (track metadata + audio) always uses librespot's keymaster default, because Spotify answers AP metadata requests made under any other client ID with `400 Bad Request`; the **Web API** (search, playlists, library) uses the custom ID when one is set (saved `spotify_client_id` setting → `DEEZY_SPOTIFY_CLIENT_ID` → default), so search draws on the user's own quota instead of the one shared by every librespot install. A custom ID therefore needs a second browser login. Changing the setting while connected logs out. The default token is stored in the OS keyring under `com.pierr.deezy` / `spotify_oauth`, the custom-ID token under `spotify_oauth_web` (with its client ID; ignored if the ID changed), both separate from the ARL; persistence is best-effort. Without a stored custom token (restart before reconnecting) the Web API falls back to the default token.
- **Callback port reuse**: the two logins reuse port 5588, so the "connected" page declares an empty icon (no `/favicon.ico` request), the second login waits 2 s, and it retries on `Auth code param not found` (a stray browser request mistaken for the auth response).
- **Diagnostics**: `spotify::debug_log` prints to stderr and appends to `deezy-debug.log` in the temp dir; `lib.rs` installs a panic hook that logs through it. The Spotify download steps, Pathfinder rejections, and Web API `/search` results are logged there — read it when a download or search misbehaves, since release builds have no console.
- **Settings are Deezer-independent**: `save_settings` no longer requires an ARL, and the Spotify source is usable with no Deezer login. `spotify_status` restores the client on startup.
- **Build pin**: `librespot-core 0.8.0`'s build script resolves `vergen-gitcl 1.0.8` + `vergen 9.1.0`, which pulls two incompatible `vergen-lib` versions and fails to compile. `[build-dependencies] vergen = "=9.0.6"` in `Cargo.toml` keeps it on `vergen-lib 0.1.6`. Do not relax that pin without testing a clean `Cargo.lock`.

### Download URL generation (legacy fallback)
When the media API (`media.deezer.com/v1/get_url`) is unavailable, the URL is constructed by AES-128-ECB encrypting a payload derived from `MD5_ORIGIN`, quality code, `SNG_ID`, and `MEDIA_VERSION`. The padding must align to a 16-byte boundary dynamically — a fixed width of 80 is insufficient for long track IDs.

---

## Theming

Themes are CSS variables applied to `document.documentElement`. Built-in values: `light`, `dark`, `system`. Custom themes are JSON files stored in the app data dir (managed by the `themes.rs` commands). The active custom theme name is stored in `settings.custom_theme`. CSS variable names follow kebab-case: `--bg-darkest`, `--accent`, `--text-primary`, etc. See `CSS_VARIABLES` in `+layout.svelte` for the full list.

---

## Internationalisation

Locale JSON files live in `src/lib/i18n/locales/` (`en`, `fr`, `de`, `es`, `it`, `pt`). The locale is initialised by `initI18n()` in `+layout.svelte` using the `locale` field from settings and kept in sync via the `currentLocale` store. Add new keys to all locale files when adding UI strings.

---

## Adding a New Tauri Command

1. Define the async function in the appropriate `commands/*.rs` module with `#[tauri::command]` (create and export a module from `commands.rs` only when adding a new domain).
2. Register it in the `invoke_handler!(tauri::generate_handler![...])` call in `lib.rs`.
3. Call it from the frontend with `invoke('command_name', { camelCaseParam: value })`.
4. For `AppState` access, add `state: tauri::State<'_, AppState>` as a parameter (order matters: Tauri state params must come before `app: AppHandle` if both are present).
