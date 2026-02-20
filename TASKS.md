# Deezy – Task Tracker

## ✅ Done

- [x] Migrate from Python CLI to Tauri + SvelteKit desktop app
- [x] Rust backend: Deezer API client (login, search, track metadata)
- [x] Rust backend: Blowfish CBC decryption for track downloads
- [x] Rust backend: Media URL resolution via `media.deezer.com/v1/get_url`
- [x] Rust backend: Quality fallback (FLAC → MP3_320 → MP3_128)
- [x] Rust backend: MP3 ID3v2.4 tag writing (title, artist, album, year, track/disc number, genre, label, cover art)
- [x] Rust backend: Settings persistence (ARL, output dir, quality) as JSON in app data dir
- [x] Rust backend: Settings validation (ARL length, directory, quality)
- [x] Rust backend: CSRF token auto-refresh on expiry
- [x] Rust backend: Folder picker dialog
- [x] Frontend: Sidebar navigation (Search, Downloads, Settings) with active download badge
- [x] Frontend: User profile display in sidebar (avatar + name)
- [x] Frontend: Auto-login on app start using saved ARL
- [x] Frontend: Search view with debounced input and rate limiting
- [x] Frontend: Search results table (cover, title, artist, album, duration)
- [x] Frontend: Download button with state indicators (idle → spinner → checkmark)
- [x] Frontend: Settings view (ARL input, folder picker, quality selector, save & login)
- [x] Frontend: Downloads view with progress bar and status text
- [x] Frontend: Download queue manager with max 3 concurrent downloads
- [x] Frontend: Rate limiter for search (2/s) and downloads (3/s)
- [x] Fix: Track download progress globally in +layout.svelte so progress bar updates even when Downloads view is not mounted
- [x] Fix: Mark download history as 100% complete on finish in downloadQueue.ts
- [x] Add "Clear history" button to Downloads view
- [x] Remove legacy Python files (pydeezer, main.py, requirements.txt)

## 📋 To Do

### High Priority
- [x] Update README.md for the new Tauri app (remove old Python instructions)
- [x] Album / playlist download support (batch download all tracks)
- [x] FLAC tag writing (currently only MP3 tags are written)
- [x] Retry failed downloads (button per item or auto-retry)
- [x] Persist download history across app restarts

### Medium Priority
- [x] Artist view / album browsing (click artist name → show discography)
- [x] Drag-and-drop reordering of download queue
- [x] Pause / resume downloads
- [x] Download folder structure options (e.g. `Artist/Album/Track.mp3`)
- [x] Dark/light theme toggle
- [x] Keyboard shortcuts (Ctrl+F for search, Escape to clear)
- [x] Notification on download complete (system toast)
- [x] Search history / recent searches

### Low Priority
- [x] Lyrics view (backend `song.getLyrics` already exists)
- [x] Audio preview / playback
- [x] Localization / i18n
- [x] Auto-update mechanism
- [x] Tray icon with minimize to tray
- [x] Export download history as CSV/JSON
- [x] Custom CSS theme support
- [x] Fix i18n startup race causing blank/black UI before locale initialization
- [x] Handle unavailable lyrics (`No lyrics id`) gracefully without surfacing backend errors
- [x] Remove sensitive auth/session values from backend terminal logs
- [x] Fix startup first-launch blank state by gating page render until app bootstrap completes
- [x] Resolve Svelte accessibility warnings in UpdateModal, ExportHistoryModal, LyricsModal, ThemeManager, and MiniPlayer
