# Deezy — Agent Reference

Deezy is a Tauri v2 desktop application that lets users download music from Deezer. The backend is written in Rust; the frontend is SvelteKit 5 (SPA mode, SSR disabled). All commands run from inside the `deezy/` subdirectory.

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

There is no test suite.

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
      commands.rs               # all #[tauri::command] handlers
      deezer/
        mod.rs                  # DeezerClient (HTTP, login, search, URL resolution)
        download.rs             # streaming download + BF decryption + tagging
        crypto.rs               # Blowfish/AES key derivation and URL encryption
        models.rs               # shared Rust structs (Serialize/Deserialize)
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
`+layout.svelte` mounts → loads download history → loads settings (calls `get_settings`) → initialises i18n → applies theme → calls `auto_login` (reads ARL from OS keyring via Rust) → sets `loggedIn` store → shows the app shell (2.2 s minimum splash).

### Download pipeline
1. Frontend calls `downloadQueueManager.addToQueue(track)` — queued if 3 downloads already active.
2. Queue pops the next item and calls `invoke('download_track', { trackId })`.
3. Rust: resolves track metadata via `song.getData` (private Deezer API), gets a CDN URL, streams the encrypted audio.
4. Decryption: every **3rd** 2048-byte chunk is Blowfish-CBC decrypted; all other chunks are written as-is.
5. Audio is written to a `.deezy.part` temp file, then moved to the final path on completion.
6. Rust emits `download-progress` Tauri events with `{ track_id, title, percent, status }` throughout.
7. After streaming, metadata tags (ID3v2.4 for MP3, Vorbis comments for FLAC) and cover art are written.
8. `+layout.svelte` listens for `download-progress` and updates the `downloads` and `downloadHistory` stores.

### IPC conventions
- All Rust commands return `Result<T, String>`. Errors bubble as rejected `invoke()` promises.
- Multi-word Rust command parameters use camelCase with `#[allow(non_snake_case)]` (e.g. `trackId`, `albumId`).
- The `get_settings` command **strips the ARL** before returning to the renderer — the ARL never crosses the IPC boundary to JavaScript.

### State management
- Global app state lives in `src/lib/stores.ts` as Svelte `writable` stores.
- `DownloadQueueManager` (singleton in `downloadQueue.ts`) is the authoritative source for download concurrency; it also updates the stores.
- Rust holds `AppState` (managed via Tauri's `.manage()`) with an `Arc<Mutex<Option<DeezerClient>>>`, a `Settings` mutex, and a cancellation map keyed by track ID.

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
`tauri.conf.json` CSP allows images only from `*.dzcdn.net` and `api.deezer.com`; media only from `*.dzcdn.net`. Adding new external image or media sources requires updating the CSP.

### Deezer uses two separate API surfaces
- **Private GW API** (`https://www.deezer.com/ajax/gw-light.php`) — used for auth (`deezer.getUserData`) and track data (`song.getData`). Requires the ARL cookie and a CSRF token (`checkForm`).
- **Legacy public API** (`https://api.deezer.com`) — used for search, album/artist/playlist metadata. No auth required for search results.

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

1. Define the async function in `commands.rs` with `#[tauri::command]`.
2. Register it in the `invoke_handler!(tauri::generate_handler![...])` call in `lib.rs`.
3. Call it from the frontend with `invoke('command_name', { camelCaseParam: value })`.
4. For `AppState` access, add `state: tauri::State<'_, AppState>` as a parameter (order matters: Tauri state params must come before `app: AppHandle` if both are present).
