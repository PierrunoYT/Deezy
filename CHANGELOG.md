# Changelog

All notable changes to Deezy are documented here.

## [0.1.0] – 2026-02-19

### Added

- **Tauri desktop app** – Full rewrite from Python CLI to Tauri 2 + SvelteKit 5 + Rust
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
- **Auto-update system** – Automatic update checks with download progress and one-click installation
- **System tray** – Tray icon with menu, minimize to tray, download status, and Ctrl+H shortcut
- **Export history** – Export download history as CSV or JSON with file picker
- **Custom themes** – Import/export custom theme files with JSON-based color definitions and theme manager UI

### Removed

- Legacy Python CLI (`main.py`, `pydeezer/`, `requirements.txt`)

### Fixed

- **Startup blank screen** – Initialize i18n with a safe default locale before first render to prevent `svelte-i18n` formatter crashes when sidebar/messages are rendered before locale setup finishes
