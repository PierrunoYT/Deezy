# Changelog

All notable changes to Deezy are documented here.

## [0.1.0] – 2026-02-19

### Added

- **Tauri desktop app** – Full rewrite from Python CLI to Tauri 2 + SvelteKit 5 + Rust
- **Rust backend** – Deezer API client with ARL-based authentication
- **Track search** – Debounced search with rate limiting (2 req/s) via Deezer public API
- **Track download** – Blowfish CBC decryption, quality fallback (FLAC → MP3_320 → MP3_128)
- **ID3v2.4 tagging** – Title, artist, album, album artist, year, track/disc number, genre, label, and 1000×1000 cover art
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

### Removed

- Legacy Python CLI (`main.py`, `pydeezer/`, `requirements.txt`)
