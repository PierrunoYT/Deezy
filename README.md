# Deezy

A desktop Deezer downloader built with [Tauri](https://tauri.app), [SvelteKit](https://kit.svelte.dev), and Rust. Search for tracks, queue downloads, and save them as MP3 320 kbps (or FLAC) with full metadata and cover art.

## Features

- **Search** – Find tracks instantly with debounced search and rate limiting
- **Download queue** – Up to 3 concurrent downloads with progress tracking
- **ID3 tags** – Title, artist, album, year, track/disc number, genre, label, and embedded cover art
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
3. Switch to **Search**, type a track name, and hit Enter
4. Click the download button on any result
5. Switch to **Downloads** to see progress

## Tech Stack

| Layer    | Technology                        |
| -------- | --------------------------------- |
| Frontend | SvelteKit 2 + Svelte 5           |
| Backend  | Rust + Tauri 2                    |
| Crypto   | Blowfish CBC (track decryption)   |
| Tags     | id3 crate (ID3v2.4)              |
| API      | Deezer private + public REST API |

## License

MIT
