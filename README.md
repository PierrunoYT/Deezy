# Deezy

A desktop Deezer downloader built with [Tauri](https://tauri.app), [SvelteKit](https://kit.svelte.dev), and Rust. Search for tracks, queue downloads, and save them as MP3 320 kbps (or FLAC) with full metadata and cover art.

## Features

### Search & Discovery
- **Multi-tab search** – Find tracks, albums, or artists with debounced search and rate limiting
- **Artist browsing** – Click any artist name to open their full discography; or use the Artists tab to search directly
- **Search history** – Recent searches dropdown with one-click re-search and privacy controls
- **Lyrics viewer** – View track lyrics in a beautiful modal with synced/plain text support
- **Audio preview** – Play 30-second previews with mini player, seek bar, and volume control

### Downloads
- **Smart queue** – Up to 3 concurrent downloads with drag-and-drop reordering
- **Album download** – Batch-download all tracks from an album with one click
- **Pause/resume** – Pause active downloads and resume them later
- **Retry failed** – One-click retry button on any failed download
- **Folder structure** – Organize downloads (Flat, Artist/Track, Artist/Album/Track, Album/Track)
- **Full metadata** – ID3v2.4 tags for MP3, Vorbis comments for FLAC (title, artist, album, year, track/disc number, genre, label, cover art)
- **Quality options** – MP3 128, MP3 320, or FLAC with automatic fallback
- **Persistent history** – Download history survives app restarts
- **Export history** – Export download history as CSV or JSON

### Customization
- **Theme system** – Light, Dark, and System themes with instant switching
- **Custom themes** – Import/export custom theme files with JSON-based color definitions
- **Internationalization** – Full support for English, Spanish, French, and German
- **Notifications** – System toast notifications for completed/failed downloads (optional)
- **Keyboard shortcuts** – Comprehensive shortcuts (Ctrl+F, Ctrl+1/2/3, Escape, Space, Shift+?)

### System Integration
- **System tray** – Minimize to tray with menu, download status, and quick controls
- **Auto-update** – Automatic update checks with one-click installation
- **Auto-login** – Reconnects on app start using your saved ARL
- **Settings persistence** – All preferences saved between sessions

## Setup

### Prerequisites

- [Node.js](https://nodejs.org) (v18+)
- [Rust](https://rustup.rs)
- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)

### Get your Deezer ARL token

1. Log into [deezer.com](https://www.deezer.com)
2. Open DevTools (`F12`) → **Application** → **Cookies** → `https://www.deezer.com`
3. Copy the value of the `arl` cookie

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

## Usage

1. **Setup** – Open the app, paste your ARL token, choose download folder and quality, then click **Save & Login**
2. **Search** – Switch to Search (Ctrl+1), type a query, and hit Enter
3. **Browse** – Toggle between **Tracks**, **Albums**, and **Artists** tabs
4. **Preview** – Click the play button to preview tracks before downloading
5. **View Lyrics** – Click the lyrics button to view song lyrics
6. **Download** – Click download on a track, or **Download All** on an album
7. **Manage Queue** – Drag to reorder, pause/resume, or remove items from the queue
8. **Monitor** – Switch to Downloads (Ctrl+2) to see progress and retry failures
9. **Customize** – Choose themes, languages, folder structure, and keyboard shortcuts in Settings (Ctrl+3)
10. **System Tray** – Minimize to tray (Ctrl+H) for background downloads

## Tech Stack

| Layer    | Technology                        |
| -------- | --------------------------------- |
| Frontend | SvelteKit 2 + Svelte 5           |
| Backend  | Rust + Tauri 2                    |
| Crypto   | Blowfish CBC (track decryption)   |
| Tags     | id3 (MP3) + metaflac (FLAC)      |
| API      | Deezer private + public REST API |

## License

MIT
