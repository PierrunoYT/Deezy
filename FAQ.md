# Wavekeep – Frequently Asked Questions

This FAQ describes the current source. Fixes listed under [Unreleased](CHANGELOG.md#unreleased) may not yet be included in the latest installer.

---

## 🔒 Security & Privacy

### How does Wavekeep handle my ARL token?

Your ARL is a sensitive session credential that can provide access to your Deezer account.

- **Stored securely when available** — Wavekeep normally stores the ARL using Windows Credential Manager, macOS Keychain, or Linux Secret Service.
- **Plaintext fallback** — If no credential is already stored and the OS credential store is unavailable, Wavekeep can store the ARL in `settings.json` so the app can still run. On Unix systems, Wavekeep restricts that file to owner-only permissions (`0600`) and refuses to load it if those permissions cannot be established. Settings displays a warning while this fallback is active.
- **Optional keyring bypass** — Set `WAVEKEEP_NO_KEYRING=1` (`true` and `yes` are also accepted; the older `DEEZY_NO_KEYRING` also works) before starting Wavekeep to keep the ARL in `settings.json` instead of the credential store (Spotify logins always use the credential store). Treat `settings.json` as a password file in this mode.
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

Spotify uses browser OAuth rather than an ARL or your Spotify password. The access and refresh tokens are normally stored in the OS credential store under `com.pierr.wavekeep` / `spotify_oauth`, and the token for your own Client ID, if you set one, under `spotify_oauth_web`, separately from the Deezer ARL. If credential persistence is unavailable, the current session can still work, but you will need to log in again after restarting.

Use **Disconnect** in Settings to remove the stored Spotify token. Changing the optional Spotify Client ID also disconnects the current session because tokens are tied to the Client ID that issued them.

### What does Wavekeep collect or send?

**Nothing goes to the developer.** The source contains no analytics, telemetry, crash reporting, update checks, or Wavekeep-operated server.

**What is sent, and where:** Wavekeep only talks to Deezer and Spotify, and to their image and audio servers, to log in, search, play previews, and download. Your Spotify login happens in your browser on Spotify's own page.

**What is stored on your computer:**

| Data | Where | How to remove it |
|---|---|---|
| Settings, including your last 20 searches | `settings.json` in the app-data folder | Clear or turn off search history in Settings |
| Download history | `download_history.json` in the app-data folder | Clear it in Downloads |
| Custom themes | `themes/` in the app-data folder | Delete them in Settings |
| Deezer ARL and Spotify logins | Your system's credential store (the ARL goes into `settings.json` only if that store is unavailable) | Log out, or disconnect Spotify, in Settings |
| Diagnostic log | `wavekeep-debug.log` in the logs folder | **Delete log** in Info & FAQ |
| Your music | The download folder you chose | Delete the files yourself; clearing history never touches them |

### Does Wavekeep keep a log? What is in it?

Yes, a diagnostic log that never leaves your computer unless you share it yourself. It lives in Wavekeep's logs folder: `%LOCALAPPDATA%\com.pierr.wavekeep\logs` on Windows, `~/Library/Logs/com.pierr.wavekeep` on macOS, and `~/.local/share/com.pierr.wavekeep/logs` (or `$XDG_DATA_HOME/com.pierr.wavekeep/logs`) on Linux. The system does not clean that folder up on its own.

- **What it records:** app start and version, the move from Deezy, whether Deezer and Spotify logins worked (account type only), quality fallbacks, the start and result of every download (and each step of Spotify downloads), Spotify rate limits, and any errors, with track IDs.
- **Track details:** track names, artists and file paths are included by default. Turn off **Track Details in Log** in Settings and they are written as `[hidden]` instead; IDs, statuses and errors are still logged.
- **Never in it:** passwords, your ARL, login tokens, and what you search for. As a safety net, every line is also cleaned before it is saved: your home folder appears as `~`, web addresses lose their query part (where tokens travel), and any long token-like string becomes `[redacted]`.
- **Size and deletion:** at 1 MB the log is renamed to `wavekeep-debug.old.log` and a new one starts. **Delete log** in Info & FAQ removes both files, and you can also delete them by hand at any time.

### Is it safe to use my Deezer account?

Wavekeep never asks for your Deezer password, but an ARL is effectively a session credential and should be protected like a password.

Using automated download software is not equivalent to normal browser use. It may violate Deezer's terms of service, and unusual download activity could result in account restrictions or suspension. Use Wavekeep at your own risk.

### What local files can Wavekeep access?

The tag editor can read or modify only MP3, FLAC, OGG, and M4A files and cover images selected through Wavekeep's native file pickers, plus audio files in the configured download folder. Changing the download folder also requires selecting it through the native folder picker. Theme imports and history exports use native open/save dialogs. Your recent searches (the last 20, unless search history is turned off in Settings) are stored in `settings.json`. Wavekeep separately maintains its settings, download history, and custom themes in its application-data folder.

---

## 🎵 Downloads & Quality

### What quality can I download in?

For **Deezer**:

| Account type | Available qualities |
|---|---|
| Free | MP3 128 kbps |
| Premium | MP3 128 kbps, MP3 320 kbps, FLAC |

Deezer Free accounts are automatically limited to MP3 128 kbps, regardless of the quality selected in Settings.

For **Spotify**, Settings offers Normal (96 kbps), High (160 kbps), Very high (320 kbps), and Lossless (FLAC for Premium). Each setting starts at its own bitrate and falls back to the others when Spotify does not supply it. At each bitrate OGG Vorbis is preferred over MP3. FLAC is only requested for Lossless; if a track offers none of the usual formats, Wavekeep takes whatever Spotify lists, such as AAC. Availability varies by track, account, and region.

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

Yes. The **music URL** field in Search accepts `deezer.com` track, album, playlist, and artist links, such as `https://www.deezer.com/track/123456`.

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

`WAVEKEEP_SPOTIFY_CLIENT_ID` (or the older `DEEZY_SPOTIFY_CLIENT_ID`) remains available as an environment override when the saved field is blank.

### The app shows a blank/black screen on startup. What do I do?

First, fully exit Wavekeep from the system tray and reopen it. If the problem continues, install the latest release again.

As a last resort, close Wavekeep and rename its application-data folder so the app can create a clean configuration:

- **Windows:** `%APPDATA%\com.pierr.wavekeep`
- **macOS:** `~/Library/Application Support/com.pierr.wavekeep`
- **Linux:** `$XDG_DATA_HOME/com.pierr.wavekeep` or `~/.local/share/com.pierr.wavekeep`

Resetting this folder removes local settings, download history, and custom themes. If Wavekeep is using plaintext fallback, it also removes the ARL. ARL and Spotify tokens stored in the OS credential store are separate and may remain there. On Linux the logs folder is inside this folder, so the log is reset too.

### Downloads are stalling or not starting. What do I do?

- Confirm that your internet connection is working.
- For Deezer, save a current ARL to verify authentication and try MP3 128 kbps if higher-quality media is unavailable.
- For Spotify, disconnect and reconnect in Settings. A custom Client ID only affects search, not downloads.
- Open **Info & FAQ** and press **Copy recent log**: the log records what each download did and the exact error. See [How do I report a problem?](#how-do-i-report-a-problem)
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

### How do I report a problem?

Open **Info & FAQ** in the sidebar, press **Copy recent log**, and paste it with a short description of what you did into a [GitHub issue](https://github.com/PierrunoYT/Wavekeep/issues) or the [Discord server](https://discord.gg/dvuWBeXSf3). **Open log folder** shows the whole file if you need more than the recent lines.

The log only leaves your computer when you share it yourself, and it never contains passwords, your ARL or login tokens. If you would rather not share track names and file paths, turn off **Track Details in Log** in Settings, press **Delete log**, and reproduce the problem before copying. You can read the log before sharing; see [Does Wavekeep keep a log?](#does-wavekeep-keep-a-log-what-is-in-it) for what it contains.

### Album covers or audio previews do not load.

Make sure you are running the latest release and that your network, DNS filter, firewall, or proxy allows HTTPS access to the selected provider's API and CDN hosts, including `api.deezer.com` and `*.dzcdn.net` for Deezer or `api.spotify.com`, `*.scdn.co`, and `*.spotifycdn.com` for Spotify. A Deezer Content Security Policy issue affecting installed builds was fixed in **v0.2.8**.

### A Deezer URL shows "Track not found" or doesn't work.

Use a full `http://` or `https://` URL from `deezer.com` with a numeric track, album, artist, or playlist ID. Supported paths include `deezer.com/track/...`, `deezer.com/album/...`, `deezer.com/artist/...`, and `deezer.com/playlist/...`, optionally with a locale segment such as `/en/`.

Short-link domains are not supported. If a valid full URL still fails, update your ARL and check whether the content is available to your account and region.

---

## 🔄 Updates

### I used Deezy. Will I lose my data when I install Wavekeep?

No. Wavekeep is the new name of Deezy. The first time it starts, it moves your settings, download history and custom themes from Deezy's data folder into its own, and moves your saved Deezer ARL and Spotify login to Wavekeep's entries in the credential store. Your download folder setting and your music files are not touched.

With the `.exe` installer, Deezy stays installed next to Wavekeep. Start Wavekeep once before you uninstall Deezy. If you uninstall Deezy first and tick **"Delete the application data"**, its uninstaller deletes the data before Wavekeep can move it. The `.msi` installer upgrades Deezy in place, so this does not come up.

### What exactly does the move from Deezy do?

Every Tauri app has an internal identifier that decides where it keeps its data and how its entries in Windows Credential Manager, macOS Keychain or Linux Secret Service are named. Deezy's identifier was `com.pierr.deezy`; Wavekeep's is `com.pierr.wavekeep`. On every start, before anything is loaded, Wavekeep:

- **Moves your data:** `settings.json`, `download_history.json` and the `themes` folder go from the old folder to the new one, for example from `%APPDATA%\com.pierr.deezy` to `%APPDATA%\com.pierr.wavekeep` on Windows. Anything already in the new folder is kept and never overwritten. The old folder is removed only once it is empty.
- **Moves your logins:** each saved credential (Deezer ARL, Spotify tokens) is copied to the new name and read back to check it, and only then is the old entry deleted, so a secret is never lost. If Wavekeep already has a newer login, the old one is simply removed. If an old entry cannot be removed, it stays and the log says so. With `WAVEKEEP_NO_KEYRING` set, the ARL is in `settings.json` and moves with it.

Once everything has moved, these steps do nothing. What happened is written to Wavekeep's log; see [Does Wavekeep keep a log?](#does-wavekeep-keep-a-log-what-is-in-it)

Things to know:

- After the move, the old Deezy app starts with default settings and no saved logins.
- On macOS, the Keychain may ask once whether Wavekeep may read the logins Deezy saved.
- On Windows, Deezy's browser cache in `%LOCALAPPDATA%\com.pierr.deezy` is not moved. You can delete that folder.
- On Linux, the old `~/.local/share/com.pierr.deezy` folder keeps Deezy's browser data, so it is not removed automatically. You can delete it.
- The startup warning appears once more, because that choice was stored with the browser data.
- The older `DEEZY_NO_KEYRING` and `DEEZY_SPOTIFY_CLIENT_ID` environment variables still work alongside the new `WAVEKEEP_` names.

### How do I update Wavekeep on Windows?

Wavekeep does not include an automatic updater. Download the latest `.exe` or `.msi` from the [GitHub Releases page](https://github.com/PierrunoYT/Wavekeep/releases/latest), fully exit Wavekeep from the system tray, and run the installer.

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

No. Check the [GitHub Releases page](https://github.com/PierrunoYT/Wavekeep/releases/latest) for new versions.

---

## ⚖️ Legal

### Is Wavekeep legal to use?

Wavekeep is provided for educational and personal use, but that does not make every use lawful. Downloading or extracting music may violate [Deezer's](https://www.deezer.com/legal/cgu) or [Spotify's](https://www.spotify.com/legal/end-user-agreement/) terms of service, copyright law, or both. A subscription does not automatically grant permission to copy or redistribute recordings.

Laws differ by jurisdiction. You are responsible for determining whether your use is permitted and for obtaining any necessary authorization. This FAQ is not legal advice. Do not redistribute downloaded content or use it commercially without the rights holders' permission.

### Does Wavekeep support the artists?

Do not assume that downloading through Wavekeep compensates artists or other rights holders. Support them through authorized streams, purchases, merchandise, or live events.

---

## 💬 Community & Support

Have a question not answered here? Join the [Discord server](https://discord.gg/dvuWBeXSf3) or open an [issue on GitHub](https://github.com/PierrunoYT/Wavekeep/issues).
