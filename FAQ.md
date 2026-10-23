# Wavekeep – Frequently Asked Questions

This FAQ describes the current source. Fixes listed under [Unreleased](CHANGELOG.md#unreleased) may not yet be included in the latest installer.

---

## 🔒 Security & Privacy

### How does Wavekeep handle my ARL token?

Your ARL is a sensitive session credential that can provide access to your Deezer account.

- **Stored securely when available** — Wavekeep normally stores the ARL using Windows Credential Manager, macOS Keychain, or Linux Secret Service.
- **Plaintext fallback** — If no credential is already stored and the OS credential store is unavailable, Wavekeep can store the ARL in `settings.json` so the app can still run. On Unix systems, Wavekeep restricts that file to owner-only permissions (`0600`) and refuses to load it if those permissions cannot be established. Settings displays a warning while this fallback is active.
- **Optional keyring bypass** — Set `WAVEKEEP_NO_KEYRING=1` (the older `WAVEKEEP_NO_KEYRING` also works) (`true` and `yes` are also accepted) before starting Wavekeep to force plaintext storage. Treat `settings.json` as a password file in this mode.
- **Automatic migration** — When the credential store becomes available again, Wavekeep moves a plaintext ARL into it and removes the ARL from `settings.json`.
- **Safe replacement** — Wavekeep authenticates a newly entered ARL before replacing the current credential. If secure storage or settings persistence fails, it preserves or restores the previous credential rather than reporting a successful replacement.
- **Not returned with settings** — After saving, `get_settings` returns an empty ARL field so the stored credential is not exposed to the renderer again.
- **Temporarily handled by the frontend** — When you paste an ARL, it exists in the application's frontend memory and is sent to the Rust backend through local Tauri IPC for authentication and storage.
- **Used only for Deezer authentication** — The current source sends authenticated requests to Deezer endpoints. It does not send the ARL to an application-operated server or analytics provider.
- **Encrypted Deezer connections** — The Deezer backend HTTP client accepts HTTPS URLs only and requires TLS 1.2 or newer.
- **Restricted Deezer hosts** — Deezer API and media requests, including redirects, are accepted only from HTTPS Deezer and Deezer CDN hosts. Spotify authentication, catalog, and audio traffic uses Spotify's own endpoints through separate clients.
- **Open source** — You can audit the relevant implementation:
  - Token storage → `src-tauri/src/settings.rs`
  - Login handling → `src-tauri/src/commands/account.rs`
  - Settings redaction → `src-tauri/src/commands/settings.rs`
  - Deezer requests → `src-tauri/src/deezer/`

### How does Wavekeep handle my Spotify login?

Spotify uses browser OAuth rather than an ARL or your Spotify password. The access and refresh tokens are normally stored in the OS credential store under `com.pierr.deezy` / `spotify_oauth`, separately from the Deezer ARL. If credential persistence is unavailable, the current session can still work, but you will need to log in again after restarting.

Use **Disconnect** in Settings to remove the stored Spotify token. Changing the optional Spotify Client ID also disconnects the current session because tokens are tied to the Client ID that issued them.

### Does Wavekeep collect analytics or telemetry?

The current source contains no analytics, telemetry, crash-reporting service, or Wavekeep-operated backend. Wavekeep still communicates with Deezer or Spotify and their content-delivery endpoints to authenticate, search, preview where available, and download content.

### Is it safe to use my Deezer account?

Wavekeep never asks for your Deezer password, but an ARL is effectively a session credential and should be protected like a password.

Using automated download software is not equivalent to normal browser use. It may violate Deezer's terms of service, and unusual download activity could result in account restrictions or suspension. Use Wavekeep at your own risk.

### What local files can Wavekeep access?

The tag editor can read or modify only MP3/FLAC files and cover images selected through Wavekeep's native file pickers, plus audio files in the configured download folder. Changing the download folder also requires selecting it through the native folder picker. Theme imports and history exports use native open/save dialogs. Wavekeep separately maintains its settings, download history, and custom themes in its application-data folder.

---

## 🎵 Downloads & Quality

### What quality can I download in?

For **Deezer**:

| Account type | Available qualities |
|---|---|
| Free | MP3 128 kbps |
| Premium | MP3 128 kbps, MP3 320 kbps, FLAC |

Deezer Free accounts are automatically limited to MP3 128 kbps, regardless of the quality selected in Settings.

For **Spotify**, Settings offers Normal (96 kbps), High (160 kbps), Very high (320 kbps), and Lossless (FLAC for Premium). OGG Vorbis and FLAC are preferred, but Wavekeep can save MP3 or M4A/AAC when those are the formats Spotify supplies. Availability varies by track, account, and region.

### Why did my Deezer download fall back to a lower quality?

Not every track is available in every quality. Wavekeep tries the selected Deezer quality first, then falls back in this order:

**FLAC → MP3 320 kbps → MP3 128 kbps**

For MP3 320 kbps, the fallback is MP3 128 kbps. Completed downloads show both the requested and actual quality when a fallback occurs. Spotify independently selects the closest available format for its quality setting.

### Where are my downloaded files saved?

Downloads are saved in the folder selected in Settings. You can organize them using one of these folder structures:

- Flat
- Artist/Track
- Artist/Album/Track
- Album/Track
- Custom template

Custom templates support `{artist}`, `{album}`, `{title}`, `{track_number}`, `{disc_number}`, `{release_date}`, and `{release_year}`. The aliases `{track}`, `{disc}`, and `{year}` are also supported. Use `/` or `\` to create folders.

### Can I download entire albums or playlists?

Yes. Click **Download All** on an album or playlist to add all its available tracks to the download queue. Wavekeep runs three downloads at the same time by default; change this with **Simultaneous Downloads** in Settings (1 to 10).

To choose individual tracks instead, click the album or playlist title. This also works for albums in an artist's discography.

### Can I download my Loved Tracks or my own playlists?

Yes. In Search, the **Loved Tracks** tab lists your favorite tracks with a **Download All** button, and **My Playlists** lists the playlists on your profile. This works for both Deezer and Spotify, including private Deezer playlists and Spotify playlists visible to your account. Click a playlist title to see its tracks. Pasted links to supported private playlists also work.

### Will Wavekeep download a track again if I already have it?

Not by default. With **Skip Existing Files** enabled in Settings, Wavekeep checks whether a supported audio file for the track already exists at the path it would save to. Deezer checks MP3 and FLAC; Spotify checks OGG, FLAC, MP3, and M4A. If a non-empty match exists, the track is skipped and shown as "Already downloaded", even if the download history was cleared.

Only the current folder structure is checked. If you change the folder structure, files saved under the previous one are not detected. Turn the setting off to always download; a second copy is then saved with a numbered name such as `Artist - Title (1).mp3`.

### Can I slow down or space out downloads?

Yes. In Settings:

- **Pause Between Downloads** waits 2–60 seconds before starting each next download.
- **Download Speed Limit** caps the total speed of all downloads. The 1411 kbit/s (FLAC) and 320 kbit/s (MP3 320) presets download each track in about the time it takes to listen to it.

Spacing out large batches makes the activity look more like normal listening. It does not remove the risk described in [Is it safe to use my Deezer account?](#is-it-safe-to-use-my-deezer-account)

### Can I paste a Deezer link to download?

Yes. The **Deezer URL** field in Search accepts `deezer.com` track, album, playlist, and artist links, such as `https://www.deezer.com/track/123456`.

Submitting a link adds its content to the download queue:

- Track links add one track.
- Album and playlist links add all returned tracks.
- Artist links add tracks from every album returned for that artist.

Large albums, playlists, or artist discographies may add many tracks to the queue. Deezer short links and links from other domains are not currently supported.

### Can I paste a Spotify link or use Spotify without Deezer?

Yes. Connect Spotify in Settings, then paste a full `open.spotify.com` track, album, artist, or playlist link into the music URL field. Spotify `spotify:` URIs are also accepted. You do not need a Deezer account or ARL. Choose Spotify as the music source to search Spotify or browse your Spotify Loved Tracks and playlists.

---

## 🛠️ Setup & Troubleshooting

### How do I get my ARL token?

1. Log into [deezer.com](https://www.deezer.com) in your browser.
2. Open DevTools (`F12`) and select **Application** in Chromium-based browsers or **Storage** in Firefox.
3. Open **Cookies** → `https://www.deezer.com`.
4. Copy the complete value of the `arl` cookie.
5. Paste it into Wavekeep's Settings and click **Save & Login**.

Treat the ARL like a password. Do not share it, post it in screenshots, or include it in bug reports.

### My ARL token expired or stopped working. What do I do?

Log into Deezer in your browser again, copy the current `arl` cookie, and save it in Wavekeep's Settings. Wavekeep automatically retries a download once when it detects an expired session; if that retry fails, you must update the ARL and log in again.

### Spotify search is rate-limited or my custom Client ID does not work. What do I do?

The default Client ID is shared by every librespot install, so its Web API search quota is often used up. Track, album, and artist searches then try Spotify Pathfinder, but Spotify currently refuses it (`403 Client/request not allowed`), so your own Client ID is the reliable fix. Downloads and track metadata use the AP session and do not depend on the Web API quota.

You can also use your own Spotify developer app to avoid the shared Web API quota. Add `http://127.0.0.1:5588/login` as its redirect URI in the Spotify Developer Dashboard, paste the Client ID into **Settings → Spotify Client ID**, and reconnect. The field saves when it loses focus; changing it while connected removes the old token. Connecting then opens the browser login twice: once for downloads, which always use the default ID because Spotify rejects track metadata requests from other IDs, and once for your app, which is used for search and your playlists. Approve both.

`WAVEKEEP_SPOTIFY_CLIENT_ID` (or the older `WAVEKEEP_SPOTIFY_CLIENT_ID`) remains available as an environment override when the saved field is blank.

### The app shows a blank/black screen on startup. What do I do?

First, fully exit Wavekeep from the system tray and reopen it. If the problem continues, install the latest release again.

As a last resort, close Wavekeep and rename its application-data folder so the app can create a clean configuration:

- **Windows:** `%APPDATA%\com.pierr.deezy`
- **macOS:** `~/Library/Application Support/com.pierr.deezy`
- **Linux:** `$XDG_DATA_HOME/com.pierr.deezy` or `~/.local/share/com.pierr.deezy`

Resetting this folder removes local settings, download history, and custom themes. If Wavekeep is using plaintext fallback, it also removes the ARL. ARL and Spotify tokens stored in the OS credential store are separate and may remain there.

### Downloads are stalling or not starting. What do I do?

- Confirm that your internet connection is working.
- For Deezer, save a current ARL to verify authentication and try MP3 128 kbps if higher-quality media is unavailable.
- For Spotify, disconnect and reconnect in Settings; if you use a custom Client ID, clear it and retry with the default.
- Pause and resume the download, or retry it from the download history after an error.
- Fully exit Wavekeep from the tray before restarting it.

Download history is persisted. Saved interrupted transfers return as paused after a restart and can be resumed manually. Pending queue entries are not persisted, so add those again after restarting. A track can also be unavailable because of catalog or regional restrictions.

When you exit normally through Wavekeep, active downloads are canceled and partial files are cleaned before the application closes. If shutdown takes longer than usual, Wavekeep may still be finishing that cleanup; avoid forcibly terminating it if possible.

### Does Resume continue from the same byte position?

No. Pause cancels the transfer and cleans its temporary file. Resume requeues the track and starts a new download from the beginning.

### Does clearing download history delete my music?

No. It removes inactive history records, not audio files. Active downloads and queued tracks with history records stay visible so their progress and final file paths remain available.

### Why does a Deezer album or playlist fail while loading?

Wavekeep follows each Deezer API page to load the complete track list. A failed later page is reported as an error rather than treated as a complete result. Try again after checking your connection. Deezer catalog loading is bounded to 100,000 entries and 1,000 follow-up pages per list, and repeated pagination links are rejected. Spotify uses its AP session for catalog track metadata instead of these Deezer pagination rules.

### How are unusual artist or track names handled in custom folders?

Only separators written in the folder template create directories. Slashes in metadata, such as `AC/DC`, are replaced with underscores; placeholder-like text inside metadata stays literal. Invalid filename characters and Windows device names are sanitized. See the [template examples](README.md#custom-folder-templates).

### Album covers or audio previews do not load.

Make sure you are running the latest release and that your network, DNS filter, firewall, or proxy allows HTTPS access to the selected provider's API and CDN hosts, including `api.deezer.com` and `*.dzcdn.net` for Deezer or `api.spotify.com`, `*.scdn.co`, and `*.spotifycdn.com` for Spotify. A Deezer Content Security Policy issue affecting installed builds was fixed in **v0.2.8**.

### A Deezer URL shows "Track not found" or doesn't work.

Use a full `http://` or `https://` URL from `deezer.com` with a numeric track, album, artist, or playlist ID. Supported paths include `deezer.com/track/...`, `deezer.com/album/...`, `deezer.com/artist/...`, and `deezer.com/playlist/...`, optionally with a locale segment such as `/en/`.

Short-link domains are not supported. If a valid full URL still fails, update your ARL and check whether the content is available to your account and region.

---

## 🔄 Updates

### I used Deezy. Will I lose my data when I install Wavekeep?

No. Wavekeep is the new name of Deezy, and it keeps your settings, download history, custom themes, saved Deezer ARL and Spotify login, and your download folder. Your music files are never touched by an update.

The one thing to watch for is the `.exe` installer: it installs Wavekeep next to the old Deezy app instead of replacing it. When you uninstall Deezy afterwards, leave **"Delete the application data"** unticked, because both apps use the same data folder and that option would wipe it. The `.msi` installer upgrades Deezy in place, so this does not come up.

### Why do some folders and settings still say "deezy"?

On purpose. Every Tauri app has an internal identifier, and Wavekeep's is still `com.pierr.deezy`. It decides two things:

- **Where your data lives:** the app-data folder for settings, download history and themes, for example `%APPDATA%\com.pierr.deezy` on Windows, `~/Library/Application Support/com.pierr.deezy` on macOS and `~/.local/share/com.pierr.deezy` on Linux.
- **Where your logins are stored:** the name of the entry in Windows Credential Manager, macOS Keychain or Linux Secret Service that holds your Deezer ARL and Spotify tokens.

If the identifier changed with the name, Wavekeep would look in a new, empty folder and a new credential entry. You would start with default settings, an empty history and no custom themes, and you would have to log in to Deezer and Spotify again. Avoiding that would need migration code that copies everything over and moves the stored credentials, which is easy to get wrong for logins. Since users never see the identifier except in these paths, keeping it is the safer choice. It may change later in a release that migrates the data properly.

For the same reason, the older `DEEZY_NO_KEYRING` and `DEEZY_SPOTIFY_CLIENT_ID` environment variables still work alongside the new `WAVEKEEP_` names.

### How do I update Wavekeep on Windows?

Wavekeep does not include an automatic updater. Download the latest `.exe` or `.msi` from the [GitHub Releases page](https://github.com/PierrunoYT/Deezy/releases/latest), fully exit Wavekeep from the system tray, and run the installer.

Installing a newer version over the existing installation should preserve settings and download history, but keeping a backup of important data is recommended.

### How do I update Wavekeep on macOS or Linux?

Prebuilt macOS and Linux packages are not currently published. If you built Wavekeep from source, update the repository and rebuild it:

```bash
cd Wavekeep
git pull
cd deezy
bun install --frozen-lockfile
bun run tauri build
```

Install the new bundle from `src-tauri/target/release/bundle/`. Use Bun to install the committed lockfile; npm can run the package scripts once dependencies are installed. See the [development guide](DEVELOPMENT.md).

### Does Wavekeep check for updates automatically?

No. Check the [GitHub Releases page](https://github.com/PierrunoYT/Deezy/releases/latest) for new versions.

---

## ⚖️ Legal

### Is Wavekeep legal to use?

Wavekeep is provided for educational and personal use, but that does not make every use lawful. Downloading or extracting music may violate [Deezer's](https://www.deezer.com/legal/cgu) or [Spotify's](https://www.spotify.com/legal/end-user-agreement/) terms of service, copyright law, or both. A subscription does not automatically grant permission to copy or redistribute recordings.

Laws differ by jurisdiction. You are responsible for determining whether your use is permitted and for obtaining any necessary authorization. This FAQ is not legal advice. Do not redistribute downloaded content or use it commercially without the rights holders' permission.

### Does Wavekeep support the artists?

Do not assume that downloading through Wavekeep compensates artists or other rights holders. Support them through authorized streams, purchases, merchandise, or live events.

---

## 💬 Community & Support

Have a question not answered here? Join the [Discord server](https://discord.gg/dvuWBeXSf3) or open an [issue on GitHub](https://github.com/PierrunoYT/Deezy/issues).
