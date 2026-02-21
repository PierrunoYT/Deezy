# Deezy

A modern desktop Deezer downloader built with [Tauri 2](https://tauri.app), [SvelteKit 5](https://kit.svelte.dev), and Rust. Search for tracks, albums, and artists, queue downloads, and save them as high-quality MP3 or FLAC with full metadata and cover art.

---

## ⚠️ Important Information

### Legal & Usage Rights

**This tool is for educational and personal use only.** By using Deezy, you acknowledge and agree to the following:

- **Deezer Account Required** – You need a Deezer account (Free or Premium) to use this application. The ARL token is tied to your account.
  - **Free accounts** are limited to MP3 128 kbps downloads
  - **Premium accounts** can download MP3 320 kbps or FLAC
- **Terms of Service** – Downloading music from Deezer may violate their [Terms of Service](https://www.deezer.com/legal/cgu). Use at your own risk.
- **Copyright Laws** – Respect copyright laws in your jurisdiction. Downloaded content is for personal use only and should not be redistributed or used commercially.
- **No Warranty** – This software is provided "as is" without warranty of any kind. The authors are not responsible for any misuse or legal consequences.
- **Educational Purpose** – This project is intended to demonstrate Tauri, Rust, and SvelteKit integration, not to encourage piracy.

**By using this software, you accept full responsibility for your actions and any consequences that may arise.**

---

## Features

### Search & Discovery
- **Multi-tab search** – Find tracks, albums, or artists with debounced search and rate limiting (2 req/s)
- **Artist browsing** – Click any artist name to open their full discography, or use the Artists tab to search directly
- **Search history** – Recent searches dropdown (up to 20 items) with one-click re-search and privacy controls
- **Lyrics viewer** – View track lyrics in a beautiful modal with synced/plain text support and graceful fallback
- **Audio preview** – Play 30-second previews with mini player, seek bar, volume control, and Space bar shortcut

### Downloads
- **Smart queue** – Up to 3 concurrent downloads with drag-and-drop reordering and priority management
- **Album download** – Batch-download all tracks from an album with one click
- **Pause/resume** – Pause active downloads and resume them later with high priority
- **Retry failed** – One-click retry button on any failed download
- **Folder structure** – Organize downloads in 4 layouts: Flat, Artist/Track, Artist/Album/Track, or Album/Track
- **Full metadata** – ID3v2.4 tags for MP3, Vorbis comments for FLAC (title, artist, album, album artist, year, track/disc number, genre, label, 1000×1000 cover art)
- **Quality options** – MP3 128, MP3 320, or FLAC with automatic fallback chain (FLAC → MP3_320 → MP3_128)
- **Quality transparency** – Completed downloads show requested quality vs actual downloaded quality when fallback occurs
- **Persistent history** – Download history saved to disk and restored on app restart
- **Export history** – Export download history as CSV or JSON with file picker dialog
- **CSRF auto-refresh** – Automatically retries downloads on token expiry with session refresh

### Customization
- **Theme system** – Light, Dark, and System themes with instant switching and OS theme detection
- **Custom themes** – Import/export custom theme files with JSON-based color definitions and theme manager UI
- **Example themes** – Three built-in custom themes (Sunset Orange, Forest Green, Midnight Blue)
- **Internationalization** – Full i18n support with English, Spanish, French, and German translations
- **Notifications** – System toast notifications for completed/failed downloads (optional toggle)
- **Keyboard shortcuts** – Comprehensive shortcuts (Ctrl+F search, Ctrl+1/2/3 navigation, Escape, Space, Shift+?, Ctrl+H minimize, Ctrl+,)
- **Accessibility improvements** – Keyboard-accessible modals/overlays, labeled icon buttons, and improved form label associations

### System Integration
- **System tray** – Minimize to tray with menu, download status indicator, and quick controls (Ctrl+H to hide)
- **Auto-update** – Automatic update checks with download progress and one-click installation via Tauri updater plugin
- **Auto-login** – Reconnects on app start using your saved ARL token
- **Settings persistence** – All preferences saved as JSON in app data directory with validation
- **Safer runtime logs** – Authentication/session token values redacted from backend logs
- **Account-aware quality** – Deezer Free accounts are restricted to MP3 128 kbps in the quality selector
- **Close to tray** – Optional setting to minimize to tray instead of closing the app

## Setup

### Prerequisites

- [Node.js](https://nodejs.org) (v18+)
- [Rust](https://rustup.rs) (latest stable)
- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) (platform-specific dependencies)

### Get your Deezer ARL token

1. Log into [deezer.com](https://www.deezer.com)
2. Open DevTools (`F12`) → **Application** (Chrome) or **Storage** (Firefox) → **Cookies** → `https://www.deezer.com`
3. Copy the value of the `arl` cookie (192-character string)

> **Note:** Your ARL token is stored locally and never shared. It expires periodically and will need to be updated.

### Build & run

```bash
cd deezy
npm install
npm run tauri dev
```

To create a production build:

```bash
npm run tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Usage

1. **Setup** – Open the app, paste your ARL token, choose download folder and quality, then click **Save & Login**
2. **Search** – Switch to Search (Ctrl+1), type a query, and hit Enter or wait for debounced search
3. **Browse** – Toggle between **Tracks**, **Albums**, and **Artists** tabs to explore different content types
4. **Preview** – Click the play button (▶) to preview tracks before downloading; use Space bar to play/pause
5. **View Lyrics** – Click the lyrics button (📄) to view song lyrics in a modal (when available)
6. **Download** – Click download button on a track, or **Download All** on an album to queue all tracks
7. **Manage Queue** – Drag to reorder pending downloads, pause/resume active ones, or remove items
8. **Monitor** – Switch to Downloads (Ctrl+2) to see real-time progress, retry failures, and export history
9. **Customize** – Choose themes, languages, folder structure, notifications, and shortcuts in Settings (Ctrl+3 or Ctrl+,)
10. **System Tray** – Minimize to tray (Ctrl+H) for background downloads; double-click tray icon to restore window

## Tech Stack

| Layer          | Technology                                                                 |
| -------------- | -------------------------------------------------------------------------- |
| Frontend       | SvelteKit 2 + Svelte 5 (runes API) + TypeScript                          |
| Backend        | Rust + Tauri 2 (desktop framework)                                        |
| HTTP Client    | reqwest (with cookie jar, streaming, and JSON support)                    |
| Crypto         | Blowfish CBC (track decryption) + AES + MD5                              |
| Audio Tags     | id3 v1.x (MP3 ID3v2.4) + metaflac v0.2 (FLAC Vorbis comments)           |
| API            | Deezer private API (`gw-light.php`) + public REST API                    |
| Async Runtime  | Tokio (full features) + futures                                           |
| Image Processing | image v0.25 (cover art embedding)                                       |
| UI Libraries   | svelte-dnd-action (drag-and-drop) + svelte-i18n (internationalization)  |
| Tauri Plugins  | dialog, notification, process, updater                                    |

## Project Structure

```
deezy/
├── src/                          # SvelteKit frontend
│   ├── lib/
│   │   ├── components/          # Svelte components (SearchView, DownloadsView, etc.)
│   │   ├── i18n/                # Internationalization (en, es, fr, de)
│   │   ├── stores.ts            # Svelte stores (loggedIn, downloads, etc.)
│   │   ├── downloadQueue.ts     # Download queue manager
│   │   ├── audioPlayer.ts       # Audio preview player
│   │   ├── keyboardShortcuts.ts # Keyboard shortcut system
│   │   └── rateLimiter.ts       # Rate limiting for API calls
│   └── routes/                  # SvelteKit routes
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── commands.rs          # Tauri commands (login, search, download, etc.)
│   │   ├── deezer/
│   │   │   ├── mod.rs           # Deezer API client
│   │   │   ├── crypto.rs        # Blowfish decryption
│   │   │   ├── download.rs      # Track download logic
│   │   │   └── models.rs        # Data models
│   │   ├── settings.rs          # Settings persistence
│   │   ├── themes.rs            # Custom theme management
│   │   ├── tray.rs              # System tray integration
│   │   └── lib.rs               # App state and setup
│   ├── Cargo.toml               # Rust dependencies
│   └── tauri.conf.json          # Tauri configuration
└── example-themes/               # Built-in custom themes (JSON)
```

## Architecture

### Frontend (Svelte 5)
- **Runes API** – Modern reactive state management with `$state`, `$effect`, and `$derived`
- **Component-based** – Modular UI components (SearchView, DownloadsView, SettingsView, etc.)
- **Download queue** – Client-side queue manager with drag-and-drop reordering and priority management
- **Rate limiting** – Separate limiters for search (2 req/s) and download (3 concurrent) operations
- **Keyboard shortcuts** – Global shortcut system with registration, categories, and help modal
- **Audio player** – Mini player with seek bar, volume control, and playback state management
- **i18n** – svelte-i18n with 4 languages, formatters for duration/fans, and locale persistence

### Backend (Rust + Tauri)
- **DeezerClient** – HTTP client with ARL authentication, CSRF token management, and session handling
- **Blowfish CBC decryption** – Track decryption using Deezer's key derivation (MD5-based)
- **Quality fallback** – Automatic fallback chain: FLAC → MP3_320 → MP3_128
- **Metadata tagging** – ID3v2.4 for MP3 (id3 crate), Vorbis comments for FLAC (metaflac crate)
- **Cover art embedding** – Downloads 1000×1000 cover art and embeds in audio files
- **Folder structure** – Configurable directory organization with automatic creation
- **System tray** – Native tray icon with menu, status updates, and window management
- **Settings persistence** – JSON-based settings storage in app data directory
- **Auto-update** – Tauri updater plugin with GitHub releases integration

## License

MIT
