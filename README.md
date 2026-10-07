# Deezy

<p align="center">
  <img src="deezy/static/logodeezy.svg" alt="Deezy Logo" width="200"/>
</p>

A modern desktop Deezer downloader. Search for tracks, albums, artists, and playlists, paste Deezer URLs for direct download, and save music as high-quality MP3 or FLAC with full metadata and cover art.

[![Discord](https://img.shields.io/badge/Discord-Join%20Server-5865F2?logo=discord&logoColor=white)](https://discord.gg/dvuWBeXSf3)
[![Release](https://img.shields.io/github/v/release/PierrunoYT/Deezy?label=latest)](https://github.com/PierrunoYT/Deezy/releases/latest)

---

## ⚠️ Important Information

**This tool is for educational and personal use only.** By using Deezy, you acknowledge and agree to the following:

- **Deezer Account Required** – You need a Deezer account (Free or Premium). The ARL token is tied to your account.
  - **Free accounts** are limited to MP3 128 kbps downloads
  - **Premium accounts** can download MP3 320 kbps or FLAC
- **Terms of Service** – Downloading music from Deezer may violate their [Terms of Service](https://www.deezer.com/legal/cgu). Use at your own risk.
- **Account Blocking Risk** – Your account may get suspended. Deezer can detect unusual download activity.
- **Copyright Laws** – Respect copyright laws in your jurisdiction. Downloaded content is for personal use only and must not be redistributed.
- **No Warranty** – This software is provided "as is". The authors are not responsible for any misuse or legal consequences.

**By using this software, you accept full responsibility for your actions.**

---

## Features

- **Search** – Find tracks, albums, artists, and playlists with debounced search
- **URL download** – Paste full `deezer.com` track, album, artist, or playlist URLs to add their content to the queue
- **Audio preview** – Play 30-second previews before downloading
- **Smart queue** – Up to 3 concurrent downloads with drag-and-drop reordering, pause/resume, and retry
- **Album & playlist download** – Download all tracks with one click
- **Quality options** – MP3 128, MP3 320, or FLAC with automatic fallback
- **Full metadata** – Title, artist, album, album artist, year, track and disc number, genre, label, and 1000×1000 cover art embedded
- **Folder structure** – Organize downloads as Flat, Artist/Track, Artist/Album/Track, Album/Track, or with a custom template
- **Tag editor** – Edit metadata and cover art on any local MP3 or FLAC file; files with an unreadable tag are reported and left untouched instead of being overwritten
- **Download history** – Persistent history with CSV/JSON export and open-in-file-manager
- **Themes** – Light, Dark, System, and fully custom JSON themes
- **Internationalization** – English, Spanish, French, German, Portuguese, and Italian
- **Keyboard shortcuts** – Ctrl+F, Ctrl+1/2/3, Ctrl+H, Space, Shift+? and more
- **System tray** – Minimize to tray with download status and quick controls
- **Manual updates** – Install Windows releases from GitHub or rebuild from source on macOS and Linux
- **Secure credentials** – ARL replacements are authenticated before storage, normally kept in the OS credential store, and redacted from `settings.json`; a clearly indicated private-file fallback is available when secure storage is unavailable

These docs describe the current source. Changes under [Unreleased](CHANGELOG.md#unreleased) may not yet be included in the latest installer.

---

## Install

### Windows

Download the latest installer from the [Releases page](https://github.com/PierrunoYT/Deezy/releases/latest):

- `.exe` (NSIS installer) — recommended
- `.msi` (MSI package)

### macOS & Linux

No pre-built binaries are available yet. Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system, plus [Bun](https://bun.sh/) and the stable Rust toolchain, then build from source:

```bash
git clone https://github.com/PierrunoYT/Deezy.git
cd Deezy/deezy
bun install --frozen-lockfile
bun run tauri build
```

Install the output from `src-tauri/target/release/bundle/` (`.dmg` on macOS, `.deb` / `.rpm` / `.AppImage` on Linux).

> **Update note:** Deezy does not currently include an automatic updater. Pull the latest source, run `bun install --frozen-lockfile`, and run `bun run tauri build` again to update a source-built installation.

### Amp development orbs

Amp automatically runs the executable `.agents/setup` script when creating a fresh orb. It installs the Linux packages and toolchains required by Tauri, installs the locked frontend dependencies, and fetches the Rust dependencies. On resume, `.agents/resume` verifies that the toolchains and frontend dependencies are ready; Deezy has no background services to restart.

---

## Get your Deezer ARL token

1. Log into [deezer.com](https://www.deezer.com)
2. Open DevTools (`F12`) → **Application** (Chrome) or **Storage** (Firefox) → **Cookies** → `https://www.deezer.com`
3. Copy the complete value of the `arl` cookie

> Treat the ARL like a password. Deezy normally stores it in Windows Credential Manager, macOS Keychain, or Linux Secret Service and excludes it from `settings.json`. If secure storage is unavailable or cannot be read (for example, no running Secret Service on Linux), Deezy can use a clearly indicated private-file fallback. A replacement is authenticated before the working credential is changed. The ARL can expire or be invalidated and may need to be updated.

---

## Usage

1. **Setup** – Paste your ARL token, choose a download folder and quality, then click **Save & Login**
2. **Search** – Switch to Search (Ctrl+1), type a query, and press Enter
3. **Paste URLs** – Paste a full `deezer.com` track, album, artist, or playlist URL to add its content to the queue
4. **Preview** – Click ▶ to preview a track; Space bar to play/pause
5. **Download** – Click the download button on a track, or **Download All** on an album or playlist
6. **Manage Queue** – Drag to reorder, pause/resume, or remove pending downloads (Ctrl+2)
7. **History** – View completed downloads, open files in Explorer/Finder, or export history
8. **Customize** – Change theme, language, folder structure, and notifications in Settings (Ctrl+3)
9. **Updates** – Download the latest Windows installer from GitHub Releases, or pull and rebuild on macOS and Linux
10. **Tray** – Minimize to tray (Ctrl+H); left-click the icon to show or hide the window

Pausing cancels the current transfer; resuming downloads the track again from the
beginning. Saved interrupted downloads appear as paused after restarting. Pending
queue entries are not persisted. Clearing history preserves active and queued
rows and does not delete downloaded files.

Album, playlist, and artist loaders follow API pagination. If a later page fails,
the operation reports an error instead of silently returning an incomplete list.

### Custom folder templates

Choose **Custom** in Settings and use a template such as:

```text
{artist}/{release_date} - {album}/{track_number} - {title}
```

Supported placeholders are `{artist}`, `{album}`, `{title}`, `{track_number}`
(`{track}`), `{disc_number}` (`{disc}`), `{release_date}`, and `{release_year}`
(`{year}`). Template `/` and `\` separators create directories; separators in
metadata are sanitized, so an artist such as `AC/DC` stays in one directory.

---

## Development

Install the Tauri prerequisites, stable Rust, Bun, and Node.js with npm. Run all
application commands from `deezy/`:

```bash
bun install --frozen-lockfile
bun run tauri dev
```

Before submitting a change:

```bash
npm test
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

See [Development guide](DEVELOPMENT.md) for the module layout, state ownership,
test coverage, and release checklist.

---

## License

MIT – see [LICENSE](LICENSE) for details.

## Resources

- [FAQ](FAQ.md) – Common questions about setup, security, and legal use
- [Changelog](CHANGELOG.md) – Full version history
- [Development guide](DEVELOPMENT.md) – Setup, architecture, testing, and releases
- [Code review](deezy/CODE_REVIEW.md) – September 2026 fixes and validation limits
