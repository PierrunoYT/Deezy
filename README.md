# Deezy

A desktop Deezer downloader built with [Tauri](https://tauri.app), [SvelteKit](https://kit.svelte.dev), and Rust. Search for tracks, queue downloads, and save them as MP3 320 kbps (or FLAC) with full metadata and cover art.

## Features

- **Search** – Find tracks or albums with debounced search and rate limiting
- **Album download** – Browse albums and batch-download all tracks with one click
- **Download queue** – Up to 3 concurrent downloads with progress tracking
- **Retry failed downloads** – One-click retry button on any failed download
- **Persistent history** – Download history survives app restarts
- **Full metadata** – ID3v2.4 tags for MP3, Vorbis comments for FLAC (title, artist, album, year, track/disc number, genre, label, cover art)
- **Quality options** – MP3 128, MP3 320, or FLAC
- **Settings** – ARL token, download folder, and quality saved between sessions
- **Auto-login** – Reconnects on app start using your saved ARL

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

1. Open the app → you'll land on **Settings**
2. Paste your ARL token, choose a download folder and quality, then click **Save & Login**
3. Switch to **Search**, type a track or album name, and hit Enter
4. Toggle between **Tracks** and **Albums** tabs
5. Click the download button on a track, or **Download All** on an album
6. Switch to **Downloads** to see progress — retry any failures with one click

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
