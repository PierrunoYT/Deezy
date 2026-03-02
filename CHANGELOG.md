# Changelog

All notable changes to Deezy are documented here.

## [Unreleased]

### Added

- **Example themes** – Added 8 new example themes to the collection: Purple Haze (vibrant purple), Ocean Teal (tropical teal), Crimson Red (bold red), Golden Amber (luxurious gold), Rose Pink (soft pink), Slate Gray (professional gray), Cherry Blossom (delicate pink), and Cyber Neon (futuristic cyan). Total of 11 example themes now available for import

### Improved

- **Theme Manager UI** – Theme cards now always display description, author, and color palette preview instead of only showing on hover; all theme details are loaded upfront for better browsing experience
- **Theme creation backend** – Updated `create_example_themes` command to include all 11 themes (previously only created 3 of the 11 available themes)

### Fixed

- **User profile image** – Fixed profile image not loading when user has no custom avatar on Deezer; now properly displays fallback avatar icon instead of attempting to load broken CDN URL with all-zero hash

### Chores

- Removed `TASKS.md` from version control and added it to `.gitignore`

## [0.2.9] - 2026-02-28

### Removed

- **Lyrics feature** – Removed lyrics viewer functionality due to poor availability from Deezer API (most tracks return "no lyrics available"). Removed `LyricsModal.svelte` component, lyrics button from track listings, `get_track_lyrics` backend command, and all related translations

### Documentation

- **README**: Added account blocking risk warning — users have reported account suspensions when using similar tools
- **README**: Added Discord server badge and link (`https://discord.gg/dvuWBeXSf3`)
- **README**: Added links to FAQ, Changelog, and LICENSE in a new "Additional Resources" section
- **FAQ.md**: Added new FAQ document covering security, downloads, setup, troubleshooting, and legal questions

## [0.2.8] - 2026-02-26

### Fixed

- **Production CSP blocking covers & audio** – Album cover images and audio preview playback worked in `tauri dev` but failed in the installed exe; the Content Security Policy was missing `media-src` for Deezer CDN audio URLs (`https://*.dzcdn.net`) and `img-src` was listing specific subdomains instead of the wildcard — CSP is now fully permissive for all `*.dzcdn.net` resources

## [0.2.7] - 2026-02-26

### Fixed

- **+page.svelte**: Fixed missing closing `</script>` tag that caused `element_unclosed` error preventing the app from loading; script tag was accidentally left open after refactoring
- **+page.svelte**: Removed redundant `role="main"` from `<main>` element (semantic HTML already implies the role)

### Improved

- **Component architecture**: Refactored all 10 Svelte components with improved code quality, type safety, and accessibility
  - Enhanced reactivity patterns using modern Svelte 5 `$effect` and `$state` runes
  - Added comprehensive TypeScript type annotations for all functions and parameters
  - Improved accessibility with proper ARIA labels, roles, and semantic HTML
  - Optimized event handlers and reduced code duplication
  - Better error handling and validation throughout
  - Enhanced performance with optimized state management and computed values
- **i18n utilities**: Refactored internationalization modules with improved type safety and functionality
  - Added comprehensive input validation for all formatter functions (handles invalid/infinite values)
  - Enhanced locale detection with fallback handling and normalization (strips region codes)
  - Added new utility functions: `isLocaleSupported`, `getSupportedLocaleOrDefault`, `changeLocale`
  - Improved error handling for invalid dates and numeric values
  - Added `formatTime` and `formatFileSize` formatters for better data presentation
  - Better code organization with constants (`DEFAULT_LOCALE`, `MILLION`, `THOUSAND`) and helper functions
  - Enhanced type definitions with `LocaleInfo` interface and readonly arrays
- **Core library modules**: Refactored all 7 core TypeScript modules with comprehensive improvements
  - **audioPlayer.ts**: Enhanced type safety, added helper methods (`getVolume`, `getDuration`, `getCurrentTime`), improved event handler management, better error handling with clamping functions
  - **downloadQueue.ts**: Optimized queue processing with helper functions, improved state management, better type safety with `DownloadStatus` type, enhanced error handling and validation
  - **keyboardShortcuts.ts**: Improved shortcut matching logic, added utility methods (`has`, `getShortcut`, `clear`), enhanced key formatting with arrow keys support, better input element detection
  - **notifications.ts**: Added notification batching with configurable limits, improved permission handling, better error truncation, added utility methods (`clearPending`, `getPendingCount`)
  - **rateLimiter.ts**: Enhanced with utility methods (`reset`, `canCallNow`, `getCallCount`), added input validation, better timing accuracy, improved documentation
  - **stores.ts**: Added comprehensive type definitions (`DownloadStatus`, `QualityOption`), exported constants (`DEFAULT_VOLUME`, `DEFAULT_THEME`), explicit `Writable` types for all stores
  - **tray.ts**: Added debounced updates to reduce API calls, improved cleanup with `destroy()` method, better tooltip building logic, enhanced type safety
- **Route and app files**: Refactored core SvelteKit routes and configuration files
  - **+layout.svelte**: Extracted helper functions for initialization, theme management, and event handling; improved code organization with constants; better cleanup with `onDestroy`; optimized theme application with separated logic
  - **+page.svelte**: Centralized keyboard shortcut registration, added type safety with `ViewType`, improved cleanup with `onDestroy`, better code organization with helper functions
  - **app.css**: Added CSS custom properties for transitions and shadows, enhanced with utility animations (`fadeIn`, `slideUp`), improved global styles with better resets, added selection styling
  - **app.html**: Enhanced meta tags (description, theme-color, color-scheme), improved accessibility with viewport-fit and role attributes, better SEO with descriptive title

### Fixed

- **DownloadsView**: Fixed `ReferenceError: unsubHistory is not defined` caused by invalid reactive statement that tried to reference `unsubHistory` before initialization; moved store subscription into `onMount` lifecycle function for proper initialization and cleanup
- **Navigation**: Fixed sidebar navigation buttons (Search, Settings) not switching views — problematic `$effect` block with store subscriptions was interfering with reactivity; moved subscriptions to `onMount` to restore proper view switching
- **SettingsView**: Fixed confusing empty ARL input field when already logged in — added visual indicator showing "ARL token is securely saved (hidden for security)" with updated placeholder text to clarify that the field is for updating the token

## [0.2.6] – 2026-02-24

### Fixed

- **DownloadsView**: Fixed invalid Svelte 5 prop syntax `{history=downloadItems}` → `history={downloadItems}` and replaced Svelte 4 `on:close` event directive with Svelte 5 `onClose` callback prop for `ExportHistoryModal`, which previously caused the modal close callback to never fire
- **SettingsView**: Fixed double-toggle bug on all three toggle switches (Notifications, Search History, System Tray) — `bind:checked` and the wrapper `onclick` were both mutating state on a single user click, causing the value to flip twice (no visible change); save functions no longer re-toggle state, label clicks stop propagation, and store subscriptions created in `onMount` are now properly unsubscribed on unmount
- **Sidebar**: Fixed user avatar fallback icon never being visible when `user.image` is falsy — `.avatar-fallback` CSS sets `display: none` by default (correct for image-error fallback) but the `{:else}` branch fallback was never overridden to `display: flex`
- **QueueView**: Fixed `dragDisabled` never being reset to `true` after a completed drag-and-drop reorder, leaving the entire queue draggable from any point rather than only from the drag handle
- **SearchView**: Fixed missing `clearTimeout(searchTimeout)` in `onMount` cleanup, which could trigger `doSearch()` after the component was unmounted
- **ExportHistoryModal**: Fixed `handleOverlayKeydown` closing the modal when Enter is pressed inside the date inputs — added `event.target !== event.currentTarget` guard so only keypresses directly on the overlay backdrop are handled
- **Rate limiter timing**: Fixed timing drift in rate limiter where `lastCallTime` was updated after the wait period, causing slight cumulative delays over many calls; now updates timestamp correctly based on whether throttling occurred
- **Keyboard shortcuts**: Simplified and fixed modifier key matching logic to require exact matches — shortcuts now properly distinguish between plain keys and modified keys (e.g., 'K' vs 'Ctrl+K')
- **Tray pause/resume**: Fixed tray pause/resume toggle only pausing queued downloads but not actively downloading tracks; added tracking of active download IDs so all downloads (active and queued) can be paused from the system tray

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

Initial release of Deezy — a modern desktop Deezer downloader built with Tauri 2, SvelteKit 2, Svelte 5, and Rust.

### Added

#### Core Features

- **Track search** – Search for tracks, albums, and artists via Deezer public API with debounced input and rate limiting
- **Track download** – Download tracks with Blowfish CBC decryption and automatic quality fallback (FLAC → MP3 320 → MP3 128)
- **Album browsing & download** – Browse album tracklists and batch-download all tracks with one click
- **Artist discovery** – Artist search results in a responsive card grid with photo, album count, and fan count; click to view full discography
- **Playlist search & browsing** – Find and browse playlists, view tracks, and batch-download
- **Audio preview** – 30-second preview playback with mini player, seek bar, volume control, and Space bar shortcut

#### File Management

- **ID3v2.4 tagging** – Automatic MP3 metadata: title, artist, album, album artist, year, track/disc number, genre, label, and 1000×1000 cover art
- **FLAC tagging** – Vorbis comments and embedded cover art
- **Folder structure options** – Configurable download organization (Flat, Artist/Track, Artist/Album/Track, Album/Track)
- **Export history** – Export download history as CSV or JSON with file picker

#### Download Queue

- **Concurrent downloads** – Up to 3 simultaneous downloads with priority sorting
- **Real-time progress** – Live progress bars via Tauri events
- **Drag-and-drop reordering** – Reorder pending downloads with visual feedback and drag handles
- **Pause/resume** – Pause active downloads and resume later with high priority
- **Retry failed downloads** – One-click retry on errored items
- **Persistent history** – Download history saved to disk and restored on app restart
- **System notifications** – Toast notifications for completed and failed downloads

#### User Interface

- **Sidebar navigation** – Search, Downloads, and Settings views with active download badge
- **User profile** – Avatar and name displayed in sidebar after login
- **Theme system** – Light, Dark, and System themes with instant switching and OS theme detection
- **Custom themes** – Import/export custom theme files with JSON-based color definitions and theme manager
- **Keyboard shortcuts** – Ctrl+F for search, Ctrl+1/2/3 for navigation, Escape to clear, Shift+? for help
- **Internationalization** – Full i18n support with English, Spanish, French, and German translations
- **System tray** – Tray icon with menu, minimize to tray, download status, and Ctrl+H shortcut

#### Settings & Security

- **ARL authentication** – Login with Deezer ARL token (show/hide toggle)
- **Auto-login** – Automatic session reconnect on app start
- **Download folder picker** – Choose where downloads are saved
- **Quality selector** – MP3 128 / MP3 320 / FLAC with Free account detection
- **Settings persistence** – Configuration saved as JSON in app data directory
- **Search history** – Recent searches dropdown with click-to-search and privacy toggle

#### Backend & Technical

- **Rust backend** – Deezer API client with ARL-based authentication and CSRF auto-refresh
- **Rate limiting** – Separate limiters for search (2 req/s) and download (3 req/s) operations
- **Quality fallback transparency** – Download history displays requested vs actual quality when fallback occurs
- **Non-blocking tag errors** – Warnings emitted to frontend when tag writing fails without blocking downloads
