# Changelog

All notable changes to Deezy are documented here.

## [Unreleased]

### Fixed

- **DownloadsView**: Fixed invalid Svelte 5 prop syntax `{history=downloadItems}` → `history={downloadItems}` and replaced Svelte 4 `on:close` event directive with Svelte 5 `onClose` callback prop for `ExportHistoryModal`, which previously caused the modal close callback to never fire
- **SettingsView**: Fixed double-toggle bug on all three toggle switches (Notifications, Search History, System Tray) — `bind:checked` and the wrapper `onclick` were both mutating state on a single user click, causing the value to flip twice (no visible change); save functions no longer re-toggle state, label clicks stop propagation, and store subscriptions created in `onMount` are now properly unsubscribed on unmount
- **Sidebar**: Fixed user avatar fallback icon never being visible when `user.image` is falsy — `.avatar-fallback` CSS sets `display: none` by default (correct for image-error fallback) but the `{:else}` branch fallback was never overridden to `display: flex`
- **LyricsModal**: Fixed `onClose` being called twice when pressing Escape — both `window.addEventListener('keydown', …)` and `onkeydown` on the backdrop handled the same key; removed the redundant global listener; also added `target === currentTarget` guard to `handleBackdropKeydown` so pressing Enter on the Synced/Plain toggle buttons no longer bubbles up and closes the modal
- **QueueView**: Fixed `dragDisabled` never being reset to `true` after a completed drag-and-drop reorder, leaving the entire queue draggable from any point rather than only from the drag handle
- **SearchView**: Fixed missing `clearTimeout(searchTimeout)` in `onMount` cleanup, which could trigger `doSearch()` after the component was unmounted
- **ExportHistoryModal**: Fixed `handleOverlayKeydown` closing the modal when Enter is pressed inside the date inputs — added `event.target !== event.currentTarget` guard so only keypresses directly on the overlay backdrop are handled

### Security

- **Settings hint XSS hardening** – Removed raw HTML rendering from the ARL hint in Settings (`{@html}` replaced with text interpolation) and updated locale strings to plain text
- **Null-byte filename sanitization** – Strips `\0` from generated filename/path components to prevent OS-level truncation issues
- **Cover-art memory guardrail** – Enforces a 10 MiB size cap for album-cover downloads to reduce memory-exhaustion risk from abnormal responses
- **Search history input cap** – Rejects oversized search-history entries (>500 bytes) to prevent unbounded settings-file growth

## [0.2.5] – 2026-02-22

### Added

- **New app logo and icons** – Replaced branding with the new `logodeezy.svg` design and regenerated Tauri icon assets (`.ico`, `.icns`, and PNG/Appx sizes) for desktop packaging
- **Startup splash screen** – Added branded startup screen with app logo and loading spinner, shown for a minimum duration before the app renders

### Fixed

- **Rust build warning cleanup** – Removed an unused trailing `downloaded` assignment in `download.rs` to eliminate `unused_assignments` warning during `tauri dev/build`
- **Startup transition polish** – Main UI now fades in smoothly after splash completion for cleaner app boot experience

## [0.2.4] – 2026-02-22

### Added

- **Languages** – Added Portuguese and Italian translations
- **Open downloaded songs from history** – Added clickable track titles in Downloads history that reveal the downloaded file in the OS file manager (Explorer/Finder/Linux file manager)

### Fixed

- **Downloads header actions styling** – Scoped status-row icon button styles to prevent them from overriding top-right header actions, fixing merged/stacked labels where `Export History`, `Clear history`, and `History` could render incorrectly
- **Windows file reveal from history** – Fixed `show_in_folder` Explorer invocation to use a Windows-safe absolute path and `explorer /select,` argument handling (including `\\?\` prefix cleanup), so clicking a downloaded track reveals the correct file instead of opening Documents
- **Free-account quality reporting** – Enforced effective quality selection in backend download command so free accounts request `MP3_128`; download history now avoids misleading `Requested MP3 320 -> Downloaded MP3 128` entries

### Documentation

- **README accuracy** – Updated Usage to include the Playlists tab and clarified settings persistence (non-secret settings in JSON, ARL in OS credential store)

## [0.2.3] – 2026-02-22

### Security

- **Credential fail-closed behavior** – `Settings::save` now requires successful OS credential-store writes and never falls back to persisting ARL in plaintext JSON
- **Renderer token exposure removed** – `get_settings` now redacts ARL before returning to the renderer process; new backend `auto_login` command handles session restore without exposing raw tokens
- **Crash hardening in crypto path** – Replaced panic-prone `expect` and unchecked slicing in download URL/decryption code with fallible error handling
- **Network/stream guardrails** – Added client connect/read timeouts and a 1 GiB per-track download safety cap to reduce hang/DoS risk
- **Safer export path handling** – Removed `unwrap()` when resolving save dialog paths during history export

### Fixed

- **MP4_RA3 file extension** – Fixed `get_quality_ext` returning `.mp3` for `MP4_RA3` quality; it now correctly returns `.mp4`, preventing ID3 tag writes to MP4 files
- **Missing Sunset Orange example theme** – `create_example_themes` now creates all three built-in themes (Sunset Orange, Forest Green, Midnight Blue) to match the example-themes directory and README
- **Dead code in download buffer handling** – Removed unreachable `buffer.len() == 2048` condition in the trailing-bytes section of `download_track`; the while-loop above guarantees `buffer.len() < 2048` at that point, so the decryption branch could never execute. Trailing partial chunks are correctly written as-is per Deezer's encryption scheme

### Documentation

- **README intro** – Corrected "SvelteKit 5" to "SvelteKit 2 + Svelte 5" in the introductory paragraph to match the actual dependency versions (`@sveltejs/kit ^2.9.0` + `svelte ^5.0.0`) and the Tech Stack table

## [0.2.2] – 2026-02-21

### Removed

- **Auto-update system** – Removed built-in auto-updater (Tauri updater plugin, update check on startup, update modal, "Check for Updates" button in Settings). Signing keys and signature generation are no longer required for builds

### Fixed

- **Download queue race condition** – Fixed race condition in queue processing that could cause downloads to stall when new items are added during processor flag transition. Added concurrency control to prevent multiple queue processors from running simultaneously
- **Keyboard shortcut logic** – Fixed inverted modifier key matching logic that prevented shortcuts without modifiers from working when any modifier key was pressed. Separated required and unwanted modifier checks for accurate shortcut detection
- **Audio player memory leak** – Added `destroy()` method to audio player manager with proper event listener cleanup. Event handlers are now stored as references and removed on cleanup to prevent memory leaks
- **Download decryption corruption** – Fixed critical bug where remaining buffer bytes after chunk processing were written without decryption check, potentially leaving encrypted data in downloaded files. Added proper decryption logic for partial chunks based on Deezer's encryption scheme
- **Download progress accuracy** – Fixed progress calculation that always incremented by 2048 bytes even when chunks were smaller, causing incorrect progress reporting. Now tracks actual chunk sizes for accurate download progress
- **Artist album track counts** – Hide "0 tracks" display when Deezer API doesn't provide track count data for artist discography albums
- **Settings accessibility** – Added ARIA roles and keyboard event handlers to toggle wrappers for notifications, search history, and system tray settings to resolve Svelte a11y warnings
- **Update checker** – Fixed "could not fetch a valid release json" error by updating release script to generate `latest.json` file with proper signature and metadata for Tauri updater plugin

## [0.2.1] – 2026-02-21

### Added

- **Playlist search & download** – New "Playlists" tab in search view to find and browse playlists, view tracks, and batch-download all tracks with one click

### Changed

- Resolve all Clippy warnings: remove needless borrow in `commands.rs`, use `.is_multiple_of()` in `download.rs`, derive `Default` for `FolderStructure` in `settings.rs`

### Fixed

- **i18n translation loading** – Fixed race condition where translation keys (e.g. `search.tabs.playlists`) were displayed as raw text instead of translated strings. Now properly awaits locale initialization using `waitLocale()` before rendering UI
- **Settings toggle buttons** – Fixed corrupted toggle switches for notifications, search history, and system tray where the purple circle overlapped text and buttons were non-functional. Restructured toggle component with separate wrapper for proper layout and click handling

## [0.2.0] – 2026-02-21

### Security

- Move ARL token storage from plaintext JSON to OS credential store (Windows Credential Manager / macOS Keychain / Linux Secret Service) with automatic migration
- Obfuscate Blowfish and AES cryptographic keys at rest in the binary (XOR deobfuscation at runtime)
- Generate real Ed25519/minisign updater signing keypair (replaces placeholder public key)
- Sanitize lyrics HTML output to prevent XSS injection from Deezer API data
- Add path traversal protection to theme load/save/delete operations (`sanitize_theme_name`)
- Restrict settings file permissions to `0600` on Unix to protect stored ARL token
- Disable `withGlobalTauri` to prevent exposing Tauri IPC on `window.__TAURI__`
- Remove all verbose `eprintln!` debug logging that could leak sensitive settings, user IDs, or session details
- Add CSV formula injection protection to download history export (`sanitize_csv_field`)
- Enforce minimum TLS 1.2 and HTTPS-only on the HTTP client
- Remove overly broad `process:default` Tauri capability, scope to `process:allow-restart` only

## [0.1.0] – 2026-02-21

### Added

- **Tauri desktop app** – Full rewrite from Python CLI to Tauri 2 + SvelteKit 2 + Svelte 5 + Rust
- **Rust backend** – Deezer API client with ARL-based authentication
- **Track search** – Debounced search with rate limiting (2 req/s) via Deezer public API
- **Track download** – Blowfish CBC decryption, quality fallback (FLAC → MP3_320 → MP3_128)
- **ID3v2.4 tagging** – MP3: title, artist, album, album artist, year, track/disc number, genre, label, and 1000×1000 cover art
- **FLAC tagging** – Vorbis comments + embedded cover art via metaflac
- **Download queue** – Up to 3 concurrent downloads with priority sorting
- **Download progress** – Real-time progress bar via Tauri events, tracked globally in layout
- **Downloads view** – Full history with cover art, progress bar, and status text
- **Clear history button** – Reset download history from the Downloads view
- **Settings view** – ARL token (show/hide), download folder picker, quality selector (MP3 128 / 320 / FLAC)
- **Settings persistence** – Saved as JSON in app data directory with validation
- **Auto-login** – Automatically reconnects on app start using saved ARL
- **CSRF auto-refresh** – Retries download on token expiry
- **Sidebar navigation** – Search, Downloads, Settings with active download badge
- **User profile** – Avatar and name displayed in sidebar with fallback icon
- **Dark theme** – Custom dark UI with purple accent
- **Rate limiting** – Separate limiters for search (2/s) and download (3/s) operations
- **Tag error handling** – Non-blocking warnings emitted to frontend when tag writing fails
- **Album search** – Tracks/Albums tab toggle in search view
- **Album download** – "Download All" button to batch-queue every track in an album
- **Artist search** – "Artists" tab in search view with dedicated search functionality via Deezer API
- **Artist cards grid** – Artist search results displayed in a responsive card grid showing artist photo, album count, and fan count
- **Artist discography view** – Clicking any artist opens a dedicated page listing all their albums with download buttons
- **Clickable artist names** – Artist names in track and album rows are now interactive links that navigate to the artist's discography
- **Back navigation** – Discography page includes a back button to return to search results while preserving the current query
- **Retry failed downloads** – One-click retry button on errored items in Downloads view
- **Persistent download history** – Saved to disk and restored on app restart
- **Drag-and-drop queue reordering** – Reorder pending downloads by dragging items with visual feedback and drag handles
- **Pause/resume downloads** – Pause active downloads and resume them later with high priority
- **Folder structure options** – Configure download organization (Flat, Artist/Track, Artist/Album/Track, Album/Track) with automatic directory creation
- **Theme system** – Light, Dark, and System themes with instant switching and OS theme detection
- **Keyboard shortcuts** – Comprehensive shortcuts (Ctrl+F for search, Ctrl+1/2/3 for navigation, Escape to clear, Shift+? for help)
- **Download notifications** – System toast notifications for completed and failed downloads with toggle in settings
- **Search history** – Recent searches dropdown with click-to-search and privacy toggle
- **Lyrics viewer** – Modal display for track lyrics with synced/plain text support and scrollable view
- **Audio preview** – 30-second preview playback with mini player, seek bar, volume control, and Space bar shortcut
- **Internationalization** – Full i18n support with English, Spanish, French, and German translations
- ~~**Auto-update system**~~ *(removed in 0.2.2)*
- **System tray** – Tray icon with menu, minimize to tray, download status, and Ctrl+H shortcut
- **Export history** – Export download history as CSV or JSON with file picker
- **Custom themes** – Import/export custom theme files with JSON-based color definitions and theme manager UI

### Removed

- Legacy Python CLI (`main.py`, `pydeezer/`, `requirements.txt`)

### Fixed

- **Startup blank screen** – Added safer i18n bootstrap with a default locale before first render to reduce early `svelte-i18n` timing crashes
- **Lyrics availability handling** – Treat Deezer `"No lyrics id ..."` responses as a normal no-lyrics case (instead of surfacing a hard backend error)
- **Sensitive log output** – Redacted sensitive auth values from backend logs (no raw ARL/CSRF/token values printed) to reduce session-information exposure
- **Startup render timing** – Gate app content rendering until initial settings/i18n/theme bootstrap flow completes to improve first-launch reliability
- **Svelte a11y warnings** – Resolved modal, icon-button, and form association warnings across Update, Export History, Lyrics, Theme Manager, and Mini Player components
- **Quality fallback transparency** – Download history now stores and displays requested quality vs actual downloaded quality when fallback occurs
- **Free tier quality restrictions** – Detect Deezer Free accounts and restrict Settings quality options to MP3 128 kbps (disable MP3 320/FLAC options)
