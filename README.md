# Deezy

<p align="center">
  <img src="deezy/static/logodeezy.png" alt="Deezy Logo" width="200"/>
</p>

A modern desktop Deezer downloader built with [Tauri 2](https://tauri.app), [SvelteKit 2](https://kit.svelte.dev) + [Svelte 5](https://svelte.dev), and Rust. Search for tracks, albums, and artists, queue downloads, and save them as high-quality MP3 or FLAC with full metadata and cover art.

[![Discord](https://img.shields.io/badge/Discord-Join%20Server-5865F2?logo=discord&logoColor=white)](https://discord.gg/dvuWBeXSf3)

---

## ⚠️ Important Information

### Legal & Usage Rights

**This tool is for educational and personal use only.** By using Deezy, you acknowledge and agree to the following:

- **Deezer Account Required** – You need a Deezer account (Free or Premium) to use this application. The ARL token is tied to your account.
  - **Free accounts** are limited to MP3 128 kbps downloads
  - **Premium accounts** can download MP3 320 kbps or FLAC
- **Terms of Service** – Downloading music from Deezer may violate their [Terms of Service](https://www.deezer.com/legal/cgu). Use at your own risk.
- **Account Blocking Risk** – Your Deezer account may get blocked. Users have reported account suspensions when using similar tools, as Deezer can detect unusual download activity and flag or ban accounts.
- **Copyright Laws** – Respect copyright laws in your jurisdiction. Downloaded content is for personal use only and should not be redistributed or used commercially.
- **No Warranty** – This software is provided "as is" without warranty of any kind. The authors are not responsible for any misuse or legal consequences.
- **Educational Purpose** – This project is intended to demonstrate Tauri, Rust, and SvelteKit integration, not to encourage piracy.

**By using this software, you accept full responsibility for your actions and any consequences that may arise.**

---

## Features

### Search & Discovery
- **Multi-tab search** – Find tracks, albums, artists, or playlists with debounced search and rate limiting (2 req/s)
- **Artist browsing** – Click any artist name to open their full discography, or use the Artists tab to search directly
- **Search history** – Recent searches dropdown (up to 20 items) with one-click re-search and privacy controls
- **Lyrics viewer** – View track lyrics in a beautiful modal with synced/plain text support and graceful fallback
- **Audio preview** – Play 30-second previews with mini player, seek bar, volume control, and Space bar shortcut

### Downloads
- **Smart queue** – Up to 3 concurrent downloads with drag-and-drop reordering via dedicated handle buttons and priority management
- **Album download** – Batch-download all tracks from an album with one click
- **Playlist download** – Browse playlist tracks and batch-download all with one click
- **Pause/resume** – Pause active downloads and resume them later with high priority
- **Retry failed** – One-click retry button on any failed download
- **Folder structure** – Organize downloads in 4 layouts: Flat, Artist/Track, Artist/Album/Track, or Album/Track
- **Full metadata** – ID3v2.4 tags for MP3, Vorbis comments for FLAC (title, artist, album, album artist, year, track/disc number, genre, label, 1000×1000 cover art)
- **Quality options** – MP3 128, MP3 320, or FLAC with automatic fallback chain (FLAC → MP3_320 → MP3_128)
- **Quality transparency** – Completed downloads show requested quality vs actual downloaded quality when fallback occurs
- **Persistent history** – Download history saved to disk and restored on app restart
- **Open in file manager** – Click a downloaded track title in history to reveal the file in Explorer/Finder/Linux file manager
- **Export history** – Export download history as CSV or JSON with file picker dialog
- **CSRF auto-refresh** – Automatically retries downloads on token expiry with session refresh

### Customization
- **Theme system** – Light, Dark, and System themes with instant switching and OS theme detection
- **Custom themes** – Import/export custom theme files with JSON-based color definitions and theme manager UI
- **Example themes** – Three built-in custom themes (Sunset Orange, Forest Green, Midnight Blue)
- **Internationalization** – Full i18n support with English, Spanish, French, German, Portuguese, and Italian translations
- **Notifications** – System toast notifications for completed/failed downloads (reliable toggle with single-fire state change)
- **Keyboard shortcuts** – Comprehensive shortcuts (Ctrl+F search, Ctrl+1/2/3 navigation, Escape, Space, Shift+?, Ctrl+H minimize, Ctrl+,)
- **Accessibility improvements** – Keyboard-accessible modals/overlays, labeled icon buttons, improved form label associations, and guarded overlay keydown handlers that ignore events bubbling from inner elements

### System Integration
- **System tray** – Minimize to tray with menu, download status indicator, and quick controls (Ctrl+H to hide)
- **Auto-login** – Reconnects on app start using your saved ARL token
- **Settings persistence** – Preferences are saved in app data (JSON for non-secret settings; ARL stored in OS credential store) with validation
- **Account-aware quality** – Deezer Free accounts are restricted to MP3 128 kbps in the quality selector
- **Close to tray** – Optional setting to minimize to tray instead of closing the app

### Security
- **Credential storage** – ARL token stored in OS credential store (Windows Credential Manager / macOS Keychain / Linux Secret Service), not in plaintext files
- **Fail-closed secret persistence** – Settings saves now fail if credential-store writes fail; ARL is never intentionally written back to JSON as a fallback
- **Renderer-safe settings API** – `get_settings` redacts ARL from renderer responses; startup session restore is handled by a backend-only auto-login command
- **TLS hardened** – Minimum TLS 1.2 and HTTPS-only enforced on all HTTP connections
- **Request resilience** – Deezer HTTP client uses connection/read timeouts to avoid indefinite hangs
- **Download safety cap** – Per-track download size is capped (1 GiB) to reduce disk exhaustion risk from abnormal streams
- **Cover-art safety cap** – Album-cover downloads are capped at 10 MiB to reduce memory exhaustion risk
- **Panic hardening** – Crypto/decryption edge cases return errors instead of panicking (`expect`/unchecked slicing removed from download-critical paths)
- **XSS protection** – All user/API-facing HTML is sanitized; CSP restricts content sources
- **Path traversal protection** – Theme filenames validated against directory traversal attacks
- **Filename null-byte sanitization** – Generated filename/path components strip null bytes and reserved characters before writing files
- **CSV injection protection** – Exported CSV fields sanitized against spreadsheet formula injection
- **Input size guardrails** – Oversized search-history entries are ignored to prevent unbounded settings-file growth
- **Minimal permissions** – Tauri capabilities scoped to only required operations

## Setup

### Prerequisites

- [Node.js](https://nodejs.org) (v18+)
- [Rust](https://rustup.rs) (latest stable)
- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) (platform-specific dependencies)

### Get your Deezer ARL token

1. Log into [deezer.com](https://www.deezer.com)
2. Open DevTools (`F12`) → **Application** (Chrome) or **Storage** (Firefox) → **Cookies** → `https://www.deezer.com`
3. Copy the value of the `arl` cookie (192-character string)

> **Note:** Your ARL token is stored securely in your OS credential store (Windows Credential Manager / macOS Keychain / Linux Secret Service) and never shared. It expires periodically and will need to be updated.

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
3. **Browse** – Toggle between **Tracks**, **Albums**, **Artists**, and **Playlists** tabs to explore different content types
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
| HTTP Client    | reqwest (cookie jar, streaming, JSON, TLS 1.2+, HTTPS-only)              |
| Crypto         | Blowfish CBC (track decryption) + AES + MD5                              |
| Audio Tags     | id3 v1.x (MP3 ID3v2.4) + metaflac v0.2 (FLAC Vorbis comments)           |
| API            | Deezer private API (`gw-light.php`) + public REST API                    |
| Async Runtime  | Tokio (full features) + futures                                           |
| Image Processing | image v0.25 (cover art embedding)                                       |
| UI Libraries   | svelte-dnd-action (drag-and-drop) + svelte-i18n (internationalization)  |
| Credentials    | keyring v3 (OS credential store: Credential Manager / Keychain / Secret Service) |
| Tauri Plugins  | dialog, notification, process                                             |

## Project Structure

```
deezy/
├── src/                          # SvelteKit frontend
│   ├── lib/
│   │   ├── components/          # Svelte components (SearchView, DownloadsView, etc.)
│   │   ├── i18n/                # Internationalization (en, es, fr, de, pt, it)
│   │   ├── stores.ts            # Svelte stores (loggedIn, downloads, etc.)
│   │   ├── downloadQueue.ts     # Download queue manager
│   │   ├── audioPlayer.ts       # Audio preview player
│   │   ├── keyboardShortcuts.ts # Keyboard shortcut system
│   │   ├── notifications.ts     # System notification manager
│   │   ├── tray.ts              # System tray integration
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
- **Runes API** – Modern reactive state management with `$state`, `$effect`, and `$derived`; all components use Svelte 5 callback props (`onClose`, `onViewChange`, …) rather than Svelte 4 event directives
- **Component-based** – 10 modular UI components with comprehensive improvements:
  - Type-safe with explicit TypeScript annotations for all functions
  - Accessible with ARIA labels, semantic HTML, and keyboard navigation
  - Optimized with helper functions and reduced code duplication
  - Proper lifecycle management with `onMount` and `onDestroy`
  - Enhanced error handling and validation throughout
- **Core libraries** – 7 refactored TypeScript modules with enhanced functionality:
  - **audioPlayer**: Event handler management, volume/time clamping, utility methods
  - **downloadQueue**: Optimized queue processing, improved state management, helper functions
  - **keyboardShortcuts**: Enhanced matching logic, utility methods, better key formatting
  - **notifications**: Batching with limits, improved permission handling, error truncation
  - **rateLimiter**: Utility methods, input validation, better timing accuracy
  - **stores**: Comprehensive type definitions, exported constants, explicit types
  - **tray**: Debounced updates, cleanup methods, optimized tooltip building
- **Routes & config** – Refactored SvelteKit routes and app configuration:
  - **+layout.svelte**: Extracted initialization helpers, improved theme management, better cleanup
  - **+page.svelte**: Centralized shortcut registration, enhanced type safety, proper lifecycle
  - **app.css**: Added CSS variables for transitions/shadows, utility animations, better global styles
  - **app.html**: Enhanced meta tags for SEO/accessibility, theme-color, color-scheme support
- **Rate limiting** – Separate limiters for search (2 req/s) and download (3 concurrent) operations with call tracking and reset capabilities
- **Keyboard shortcuts** – Global shortcut system with registration, categories, and help modal; modals guard all keydown handlers with `target === currentTarget` to prevent inner-element keypresses from closing them unexpectedly
- **Audio player** – Mini player with seek bar, volume control, playback state management, and proper resource cleanup
- **i18n** – svelte-i18n with 6 languages and robust formatting utilities:
  - Locale detection with fallback handling and normalization
  - Comprehensive formatters: duration, numbers, dates, time, file sizes, fan counts
  - Input validation for all formatters (handles invalid/infinite values gracefully)
  - Type-safe locale management with `LocaleInfo` interface

### Backend (Rust + Tauri)
- **DeezerClient** – HTTP client with ARL authentication, CSRF token management, and session handling
- **Blowfish CBC decryption** – Track decryption using Deezer's key derivation (MD5-based)
- **Quality fallback** – Automatic fallback chain: FLAC → MP3_320 → MP3_128
- **Metadata tagging** – ID3v2.4 for MP3 (id3 crate), Vorbis comments for FLAC (metaflac crate)
- **Cover art embedding** – Downloads 1000×1000 cover art and embeds in audio files
- **Folder structure** – Configurable directory organization with automatic creation
- **System tray** – Native tray icon with menu, status updates, and window management
- **Credential storage** – ARL stored in OS credential store via keyring crate; settings file contains no secrets

## License

MIT – see [LICENSE](LICENSE) for details.

## Additional Resources

- [FAQ](FAQ.md) – Common questions about security, downloads, setup, and legal use
- [Changelog](CHANGELOG.md) – Full version history and release notes
