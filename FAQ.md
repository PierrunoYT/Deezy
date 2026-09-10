# Deezy – Frequently Asked Questions

This FAQ describes the current source. Fixes listed under [Unreleased](CHANGELOG.md#unreleased) may not yet be included in the latest installer.

---

## 🔒 Security & Privacy

### How does Deezy handle my ARL token?

Your ARL is a sensitive session credential that can provide access to your Deezer account.

- **Stored securely when available** — Deezy normally stores the ARL using Windows Credential Manager, macOS Keychain, or Linux Secret Service.
- **Plaintext fallback** — If no credential is already stored and the OS credential store is unavailable, Deezy can store the ARL in `settings.json` so the app can still run. On Unix systems, Deezy restricts that file to owner-only permissions (`0600`) and refuses to load it if those permissions cannot be established. Settings displays a warning while this fallback is active.
- **Optional keyring bypass** — Set `DEEZY_NO_KEYRING=1` (`true` and `yes` are also accepted) before starting Deezy to force plaintext storage. Treat `settings.json` as a password file in this mode.
- **Automatic migration** — When the credential store becomes available again, Deezy moves a plaintext ARL into it and removes the ARL from `settings.json`.
- **Safe replacement** — Deezy authenticates a newly entered ARL before replacing the current credential. If secure storage or settings persistence fails, it preserves or restores the previous credential rather than reporting a successful replacement.
- **Not returned with settings** — After saving, `get_settings` returns an empty ARL field so the stored credential is not exposed to the renderer again.
- **Temporarily handled by the frontend** — When you paste an ARL, it exists in the application's frontend memory and is sent to the Rust backend through local Tauri IPC for authentication and storage.
- **Used only for Deezer authentication** — The current source sends authenticated requests to Deezer endpoints. It does not send the ARL to an application-operated server or analytics provider.
- **Encrypted network connections** — Deezy's backend HTTP client accepts HTTPS URLs only and requires TLS 1.2 or newer.
- **Restricted remote hosts** — Backend API and media requests, including redirects, are accepted only from HTTPS Deezer and Deezer CDN hosts.
- **Open source** — You can audit the relevant implementation:
  - Token storage → `src-tauri/src/settings.rs`
  - Login handling → `src-tauri/src/commands/account.rs`
  - Settings redaction → `src-tauri/src/commands/settings.rs`
  - Deezer requests → `src-tauri/src/deezer/`

### Does Deezy collect analytics or telemetry?

The current source contains no analytics, telemetry, crash-reporting service, or Deezy-operated backend. Deezy still communicates with Deezer and its content-delivery endpoints to authenticate, search, preview, and download content.

### Is it safe to use my Deezer account?

Deezy never asks for your Deezer password, but an ARL is effectively a session credential and should be protected like a password.

Using automated download software is not equivalent to normal browser use. It may violate Deezer's terms of service, and unusual download activity could result in account restrictions or suspension. Use Deezy at your own risk.

### What local files can Deezy access?

The tag editor can read or modify only MP3/FLAC files and cover images selected through Deezy's native file pickers, plus audio files in the configured download folder. Changing the download folder also requires selecting it through the native folder picker. Theme imports and history exports use native open/save dialogs. Deezy separately maintains its settings, download history, and custom themes in its application-data folder.

---

## 🎵 Downloads & Quality

### What quality can I download in?

| Account type | Available qualities |
|---|---|
| Free | MP3 128 kbps |
| Premium | MP3 128 kbps, MP3 320 kbps, FLAC |

Free accounts are automatically limited to MP3 128 kbps, regardless of the quality selected in Settings. Availability can also vary by track and region.

### Why did my download fall back to a lower quality?

Not every track is available in every quality. Deezy tries the selected quality first, then falls back in this order:

**FLAC → MP3 320 kbps → MP3 128 kbps**

For MP3 320 kbps, the fallback is MP3 128 kbps. Completed downloads show both the requested and actual quality when a fallback occurs.

### Where are my downloaded files saved?

Downloads are saved in the folder selected in Settings. You can organize them using one of these folder structures:

- Flat
- Artist/Track
- Artist/Album/Track
- Album/Track
- Custom template

Custom templates support `{artist}`, `{album}`, `{title}`, `{track_number}`, `{disc_number}`, `{release_date}`, and `{release_year}`. The aliases `{track}`, `{disc}`, and `{year}` are also supported. Use `/` or `\` to create folders.

### Can I download entire albums or playlists?

Yes. Click **Download All** on an album or playlist to add all its available tracks to the download queue. Deezy runs up to three downloads concurrently.

### Can I paste a Deezer link to download?

Yes. The **Deezer URL** field in Search accepts `deezer.com` track, album, playlist, and artist links, such as `https://www.deezer.com/track/123456`.

Submitting a link adds its content to the download queue:

- Track links add one track.
- Album and playlist links add all returned tracks.
- Artist links add tracks from every album returned for that artist.

Large albums, playlists, or artist discographies may add many tracks to the queue. Deezer short links and links from other domains are not currently supported.

---

## 🛠️ Setup & Troubleshooting

### How do I get my ARL token?

1. Log into [deezer.com](https://www.deezer.com) in your browser.
2. Open DevTools (`F12`) and select **Application** in Chromium-based browsers or **Storage** in Firefox.
3. Open **Cookies** → `https://www.deezer.com`.
4. Copy the complete value of the `arl` cookie.
5. Paste it into Deezy's Settings and click **Save & Login**.

Treat the ARL like a password. Do not share it, post it in screenshots, or include it in bug reports.

### My ARL token expired or stopped working. What do I do?

Log into Deezer in your browser again, copy the current `arl` cookie, and save it in Deezy's Settings. Deezy automatically retries a download once when it detects an expired session; if that retry fails, you must update the ARL and log in again.

### The app shows a blank/black screen on startup. What do I do?

First, fully exit Deezy from the system tray and reopen it. If the problem continues, install the latest release again.

As a last resort, close Deezy and rename its application-data folder so the app can create a clean configuration:

- **Windows:** `%APPDATA%\com.pierr.deezy`
- **macOS:** `~/Library/Application Support/com.pierr.deezy`
- **Linux:** `$XDG_DATA_HOME/com.pierr.deezy` or `~/.local/share/com.pierr.deezy`

Resetting this folder removes local settings, download history, and custom themes. If Deezy is using plaintext fallback, it also removes the ARL. An ARL stored in the OS credential store is separate and may remain there.

### Downloads are stalling or not starting. What do I do?

- Confirm that your internet connection is working.
- Open Settings and save a current ARL to verify that Deezer authentication succeeds.
- Try the same track at MP3 128 kbps in case higher-quality media is unavailable.
- Pause and resume the download, or retry it from the download history after an error.
- Fully exit Deezy from the tray before restarting it.

Download history is persisted. Saved interrupted transfers return as paused after a restart and can be resumed manually. Pending queue entries are not persisted, so add those again after restarting. A track can also be unavailable because of catalog or regional restrictions.

When you exit normally through Deezy, active downloads are canceled and partial files are cleaned before the application closes. If shutdown takes longer than usual, Deezy may still be finishing that cleanup; avoid forcibly terminating it if possible.

### Does Resume continue from the same byte position?

No. Pause cancels the transfer and cleans its temporary file. Resume requeues the track and starts a new download from the beginning.

### Does clearing download history delete my music?

No. It removes inactive history records, not audio files. Active downloads and queued tracks with history records stay visible so their progress and final file paths remain available.

### Why does an album or playlist fail while loading?

Deezy follows each API page to load the complete track list. A failed later page is reported as an error rather than treated as a complete result. Try again after checking your connection. Catalog loading is bounded to 100,000 entries and 1,000 follow-up pages per list, and repeated pagination links are rejected.

### How are unusual artist or track names handled in custom folders?

Only separators written in the folder template create directories. Slashes in metadata, such as `AC/DC`, are replaced with underscores; placeholder-like text inside metadata stays literal. Invalid filename characters and Windows device names are sanitized. See the [template examples](README.md#custom-folder-templates).

### Album covers or audio previews do not load.

Make sure you are running the latest release and that your network, DNS filter, firewall, or proxy allows HTTPS access to `api.deezer.com` and `*.dzcdn.net`. A Content Security Policy issue affecting installed builds was fixed in **v0.2.8**.

### A Deezer URL shows "Track not found" or doesn't work.

Use a full `http://` or `https://` URL from `deezer.com` with a numeric track, album, artist, or playlist ID. Supported paths include `deezer.com/track/...`, `deezer.com/album/...`, `deezer.com/artist/...`, and `deezer.com/playlist/...`, optionally with a locale segment such as `/en/`.

Short-link domains are not supported. If a valid full URL still fails, update your ARL and check whether the content is available to your account and region.

---

## 🔄 Updates

### How do I update Deezy on Windows?

Deezy does not include an automatic updater. Download the latest `.exe` or `.msi` from the [GitHub Releases page](https://github.com/PierrunoYT/Deezy/releases/latest), fully exit Deezy from the system tray, and run the installer.

Installing a newer version over the existing installation should preserve settings and download history, but keeping a backup of important data is recommended.

### How do I update Deezy on macOS or Linux?

Prebuilt macOS and Linux packages are not currently published. If you built Deezy from source, update the repository and rebuild it:

```bash
cd Deezy
git pull
cd deezy
bun install --frozen-lockfile
bun run tauri build
```

Install the new bundle from `src-tauri/target/release/bundle/`. Use Bun to install the committed lockfile; npm can run the package scripts once dependencies are installed. See the [development guide](DEVELOPMENT.md).

### Does Deezy check for updates automatically?

No. Check the [GitHub Releases page](https://github.com/PierrunoYT/Deezy/releases/latest) for new versions.

---

## ⚖️ Legal

### Is Deezy legal to use?

Deezy is provided for educational and personal use, but that does not make every use lawful. Downloading or extracting music may violate [Deezer's Terms of Service](https://www.deezer.com/legal/cgu), copyright law, or both. A Deezer subscription does not automatically grant permission to copy or redistribute recordings.

Laws differ by jurisdiction. You are responsible for determining whether your use is permitted and for obtaining any necessary authorization. This FAQ is not legal advice. Do not redistribute downloaded content or use it commercially without the rights holders' permission.

### Does Deezy support the artists?

Do not assume that downloading through Deezy compensates artists or other rights holders. Support them through authorized streams, purchases, merchandise, or live events.

---

## 💬 Community & Support

Have a question not answered here? Join the [Discord server](https://discord.gg/dvuWBeXSf3) or open an [issue on GitHub](https://github.com/PierrunoYT/Deezy/issues).
