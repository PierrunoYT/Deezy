# Wavekeep FAQ

This FAQ describes Wavekeep 0.3.0. Use the headings to jump to what you need.

---

## Read this first

**Wavekeep is an unofficial tool.** It is not made, approved or supported by Deezer or Spotify.

- **It can get your account banned.** Downloading with software like this breaks Deezer's and Spotify's terms of service. They can warn, restrict, suspend or ban an account, and they can do it without notice. Spacing out downloads (see [Can I slow down downloads?](#can-i-slow-down-or-space-out-downloads)) makes the activity look more like normal listening, but it does not remove the risk.
- **You use it at your own risk.** Wavekeep comes with no warranty. The developer takes no responsibility for banned or restricted accounts, lost playlists, subscriptions or purchases, or anything else that happens because you used it. Do not use an account you cannot afford to lose.
- **Copyright is your responsibility.** Only download music you have the right to copy. See [Is Wavekeep legal to use?](#is-wavekeep-legal-to-use)

If you do not accept this, do not use Wavekeep.

---

## 🚀 Getting started

### What is Wavekeep?

A desktop app (Windows installers are published; macOS and Linux can be built from source) that searches Deezer and Spotify and downloads tracks, albums and playlists with tags and cover art. It was called Deezy before version 0.3.0.

### How do I get my Deezer ARL token?

1. Log into [deezer.com](https://www.deezer.com) in your browser.
2. Open DevTools (`F12`) and select **Application** in Chromium-based browsers or **Storage** in Firefox.
3. Open **Cookies** → `https://www.deezer.com`.
4. Copy the complete value of the `arl` cookie.
5. Paste it into Wavekeep's Settings and click **Save & Login**.

Treat the ARL like a password. Do not share it, post it in screenshots, or include it in bug reports.

### Can I use Spotify without Deezer?

Yes. Connect Spotify in Settings (a browser login on Spotify's own page), then choose Spotify as the music source. You do not need a Deezer account or ARL. You can also paste a full `open.spotify.com` track, album, artist or playlist link, or a `spotify:` URI, into the music URL field.

### Can I paste a link to download?

Yes. The **music URL** field in Search accepts `deezer.com` and `open.spotify.com` links:

- Track links add one track.
- Album and playlist links add all their tracks.
- Artist links add the tracks of every album returned for that artist, after asking you to confirm the number of albums.

Deezer links must be full `http(s)://` links with a numeric ID, such as `https://www.deezer.com/track/123456`, optionally with a locale like `/en/`. Short links and other domains are not supported.

---

## 🎵 Downloads & quality

### What quality can I download in?

**Deezer**

| Account type | Available qualities |
|---|---|
| Free | MP3 128 kbps |
| Premium | MP3 128 kbps, MP3 320 kbps, FLAC |

Free accounts are always limited to MP3 128 kbps, whatever you choose in Settings.

**Spotify:** Settings offers Normal (96 kbps), High (160 kbps), Very high (320 kbps) and Lossless (FLAC, Premium). Each setting starts at its own bitrate and falls back to the others when Spotify does not supply it. At each bitrate OGG Vorbis is preferred over MP3. FLAC is only requested for Lossless; if a track offers none of the usual formats, Wavekeep takes whatever Spotify lists, such as AAC. Wavekeep does not check your Spotify account type, so what a Free account gets depends on what Spotify serves. Availability varies by track, account and region.

### Why did my Deezer download fall back to a lower quality?

Not every track exists in every quality. Wavekeep tries your selected Deezer quality first and then falls back: **FLAC → MP3 320 kbps → MP3 128 kbps** (MP3 320 falls back to MP3 128). Completed downloads show the requested and the actual quality when a fallback happened. Spotify picks the closest available format for its own setting.

### Where are my downloads saved?

In the folder chosen in Settings (by default a `Wavekeep` folder inside your Music folder). Folder structures: Flat, Artist/Track, Artist/Album/Track, Album/Track, or a custom template.

Custom templates support `{artist}`, `{album}`, `{title}`, `{track_number}`, `{disc_number}`, `{release_date}` and `{release_year}`, plus the aliases `{track}`, `{disc}` and `{year}`. Use `/` or `\` to make folders.

Slashes inside metadata, such as `AC/DC`, become underscores and never create folders. Invalid filename characters and Windows device names are cleaned, and file and folder names are shortened to 180 bytes (the extension is always kept). See the [template examples](README.md#custom-folder-templates).

### Can I download whole albums, playlists, Loved Tracks or my own playlists?

Yes. Click **Download All** on an album or playlist, or click its title to pick individual tracks (this also works for albums in an artist's discography). In Search, the **Loved Tracks** tab lists your favourites with **Download All**, and **My Playlists** lists your profile's playlists. This works for Deezer, including private playlists, and for Spotify. Pasted links to supported private playlists work too.

Wavekeep runs three downloads at the same time by default; change **Simultaneous Downloads** in Settings (1 to 10).

### Will Wavekeep download a track again if I already have it?

Not by default. With **Skip Existing Files** on, Wavekeep checks whether a supported audio file already exists at the path it would save to (Deezer: MP3 and FLAC; Spotify: OGG, FLAC, MP3 and M4A). A non-empty match is skipped and shown as "Already downloaded", even if you cleared the history.

Only the current folder structure is checked, so files saved under an earlier structure are not found. With the setting off, a second copy is saved with a numbered name such as `Artist - Title (1).mp3`.

### Can I slow down or space out downloads?

Yes, in Settings:

- **Pause Between Downloads** waits 2 to 60 seconds before each next download starts.
- **Download Speed Limit** caps the total speed of all downloads. The 1411 kbit/s (FLAC) and 320 kbit/s (MP3 320) presets download each track in about the time it takes to listen to it.

This is not protection against bans. See [Read this first](#read-this-first).

### Does Resume continue from the same byte position?

No. Pause cancels the transfer and removes its temporary file; Resume starts the track again from the beginning.

### Does clearing download history delete my music?

No. It removes inactive history records, not audio files. Active and queued downloads stay visible.

---

## 🔒 Privacy & security

### Can my account get banned? Is it safe to use my account?

See [Read this first](#read-this-first): yes, it can, and you use Wavekeep at your own risk. Wavekeep never asks for your Deezer or Spotify password, but an ARL is a session credential and protecting it matters as much as protecting a password.

### What does Wavekeep collect or send?

**Nothing goes to the developer.** The source has no analytics, telemetry, crash reporting, update checks or Wavekeep-operated server.

**What is sent, and where:** Wavekeep only talks to Deezer and Spotify, and to their image and audio servers, to log in, search, play previews and download. Your Spotify login happens in your browser on Spotify's own page.

**What is stored on your computer:**

| Data | Where | How to remove it |
|---|---|---|
| Settings, including your last 20 searches | `settings.json` in the app-data folder | Clear or turn off search history in Settings |
| Download history | `download_history.json` in the app-data folder | Clear it in Downloads |
| Custom themes | `themes/` in the app-data folder | Delete them in Settings |
| Deezer ARL and Spotify logins | Your system's credential store (the ARL goes into `settings.json` only if that store is unavailable) | Log out, or disconnect Spotify, in Settings |
| Diagnostic log | `wavekeep-debug.log` in the logs folder | **Delete log** in Info & FAQ |
| Your music | The download folder you chose | Delete the files yourself; clearing history never touches them |

### How is my ARL stored?

- **In the OS credential store** (Windows Credential Manager, macOS Keychain, Linux Secret Service) when it is available.
- **Plaintext fallback:** if no credential is stored yet and the credential store is unavailable, the ARL is kept in `settings.json` so the app still works (owner-only permissions on Unix). It moves into the credential store, and out of the file, once the store works again. Set `WAVEKEEP_NO_KEYRING=1` (`true` and `yes` also work; the older `DEEZY_NO_KEYRING` too) before starting to keep the ARL in `settings.json` on purpose.
- **Never given back to the interface:** settings returned to the window have an empty ARL field, and only a verified login can change the stored ARL. A new ARL is checked with Deezer before it replaces the old one, and a failed save restores the previous one.
- **Used only for Deezer:** requests go to Deezer's HTTPS hosts only (TLS 1.2 or newer, redirects included) and never to a Wavekeep server. While you paste it, it passes through the app's window and a local connection to the backend.
- **Open source:** see `src-tauri/src/settings.rs`, `commands/account.rs`, `commands/settings.rs` and `deezer/`.

### How is my Spotify login stored?

Spotify uses a browser login, not an ARL or your Spotify password. The tokens are normally kept in the OS credential store under `com.pierr.wavekeep` / `spotify_oauth` (and `spotify_oauth_web` for your own Client ID), separately from the ARL. If the credential store is unavailable, the current session still works but you must log in again after a restart. **Disconnect** in Settings removes the stored token. Changing the Spotify Client ID disconnects you, because tokens belong to the Client ID that issued them.

### Does Wavekeep keep a log? What is in it?

Yes, a diagnostic log that never leaves your computer unless you share it yourself. It lives in the logs folder: `%LOCALAPPDATA%\com.pierr.wavekeep\logs` on Windows, `~/Library/Logs/com.pierr.wavekeep` on macOS, and `~/.local/share/com.pierr.wavekeep/logs` (or `$XDG_DATA_HOME/com.pierr.wavekeep/logs`) on Linux. The system does not clean that folder up on its own.

- **What it records:** app start and version, the move from Deezy, whether Deezer and Spotify logins worked (account type only), quality fallbacks, the start and result of every download (and each step of Spotify downloads), Spotify rate limits, and errors, with track IDs.
- **Track details:** track names, artists and file paths are included by default. Turn off **Track Details in Log** in Settings and they are written as `[hidden]`; IDs, statuses and errors are still logged.
- **Never in it:** passwords, your ARL, login tokens, and what you search for. As a safety net, every line is cleaned before it is saved: your home folder appears as `~`, web addresses lose their query part, and long token-like strings become `[redacted]`.
- **Size and deletion:** at 1 MB the log is renamed to `wavekeep-debug.old.log` and a new one starts. **Delete log** in Info & FAQ removes both files, and you can delete them by hand.

### What local files can Wavekeep access?

The Tag Editor can read or change only MP3, FLAC, OGG and M4A files and cover images you pick in Wavekeep's own file dialogs, plus audio files in your download folder. Before saving, it copies the file next to itself (a hidden `.wavekeep.bak` file) and puts the original back if the save fails. The download folder can only be changed through the folder dialog. Theme imports and history exports use native open and save dialogs. Settings, download history and custom themes live in Wavekeep's app-data folder.

---

## 🛠️ Troubleshooting

### My ARL expired or stopped working.

Log into Deezer in your browser again, copy the current `arl` cookie and save it in Settings. Wavekeep retries a download once by itself when it sees an expired session; if that fails, update the ARL and log in again.

### Spotify search is rate-limited, or my custom Client ID does not work.

The default Client ID is shared by every librespot install, so its Web API search quota is often used up. Wavekeep then tries Spotify's Pathfinder search, but Spotify currently refuses it (`403 Client/request not allowed`), so **your own Client ID is the reliable fix**. Downloads and track metadata use a different connection and do not depend on that quota.

1. In the Spotify Developer Dashboard, create an app and add `http://127.0.0.1:5588/login` as a redirect URI.
2. Paste its Client ID into **Settings → Spotify Client ID** (it saves when the field loses focus; changing it while connected disconnects you).
3. Reconnect. The browser login opens **twice**: once for downloads (which always use the default ID, because Spotify rejects track requests from other IDs) and once for your app (search and your playlists). Approve both.

`WAVEKEEP_SPOTIFY_CLIENT_ID` (or the older `DEEZY_SPOTIFY_CLIENT_ID`) works as an override when the saved field is blank. A Client ID from a Spotify app in development mode may carry restrictions set on Spotify's side that Wavekeep cannot see.

### The Spotify login window never finishes.

The login waits 5 minutes and then gives up and frees its port (5588), so **Connect** works again. If it keeps failing, close other programs that might use port 5588 and try again.

### Downloads are stalling or not starting.

- Check your internet connection.
- Deezer: save a current ARL and try MP3 128 kbps if higher quality is unavailable.
- Spotify: disconnect and reconnect in Settings. A custom Client ID only affects search, not downloads.
- Open **Info & FAQ** and press **Copy recent log**: it records what each download did and the exact error (see [How do I report a problem?](#how-do-i-report-a-problem)).
- Pause and resume the download, or retry it from the download history after an error.
- Fully exit Wavekeep from the tray before restarting it.

Download history is saved: interrupted transfers come back paused after a restart and can be resumed by hand. Queued tracks that had not started are not saved, so add them again. A track can also be unavailable because of catalog or regional limits.

When you quit through Wavekeep, active downloads are cancelled and their partial files cleaned (this waits at most 30 seconds). A hard crash or a forced kill can leave an unfinished `.wavekeep.part` file in the download folder, which you can delete.

### A Deezer album or playlist fails while loading.

Wavekeep follows every Deezer API page to load the whole track list, and a failed later page is an error rather than a shorter list. Check your connection and try again. Lists are limited to 100,000 entries and 1,000 follow-up pages. Spotify catalog loading works differently and does not use these limits.

### A Deezer URL shows "Track not found" or does not work.

Use a full `http://` or `https://` link from `deezer.com` with a numeric ID and one of `/track/`, `/album/`, `/artist/` or `/playlist/`. Short links are not supported. If a full link still fails, update your ARL and check that the content is available to your account and region.

### Album covers or audio previews do not load.

Use the latest release and make sure your network, DNS filter, firewall or proxy allows HTTPS to the provider's hosts: `api.deezer.com` and `*.dzcdn.net` for Deezer, `api.spotify.com`, `*.scdn.co` and `*.spotifycdn.com` for Spotify.

### The app shows a blank or black screen on startup.

Fully exit Wavekeep from the system tray and reopen it. If that does not help, install the latest release again. As a last resort, close Wavekeep and rename its app-data folder so it creates a clean configuration:

- **Windows:** `%APPDATA%\com.pierr.wavekeep`
- **macOS:** `~/Library/Application Support/com.pierr.wavekeep`
- **Linux:** `$XDG_DATA_HOME/com.pierr.wavekeep` or `~/.local/share/com.pierr.wavekeep`

This removes local settings, download history and custom themes, and the ARL too if Wavekeep was using the plaintext fallback. Logins in the OS credential store are separate and may remain. On Linux the logs folder is inside this folder, so the log is reset as well.

### How do I report a problem?

Open **Info & FAQ** in the sidebar, press **Copy recent log**, and paste it with a short description of what you did into a [GitHub issue](https://github.com/PierrunoYT/Wavekeep/issues) or the [Discord server](https://discord.gg/dvuWBeXSf3). **Open log folder** shows the whole file.

The log only leaves your computer when you share it, and it never contains passwords, your ARL or login tokens. To keep track names and file paths out of it, turn off **Track Details in Log** in Settings, press **Delete log**, reproduce the problem, then copy. You can read the log before sharing; see [Does Wavekeep keep a log?](#does-wavekeep-keep-a-log-what-is-in-it) for what it contains.

---

## 🔄 Updates & moving from Deezy

### How do I update Wavekeep?

There is no automatic updater, and Wavekeep never checks for updates. Download the latest `.exe` or `.msi` from the [GitHub Releases page](https://github.com/PierrunoYT/Wavekeep/releases/latest), fully exit Wavekeep from the tray, and run the installer. Settings and history should be kept, but back up anything important. The installers are not code-signed, so Windows SmartScreen may warn.

Prebuilt macOS and Linux packages are not published. If you built from source:

```bash
cd Wavekeep
git pull
cd deezy
bun install --frozen-lockfile
bun run tauri build
```

Then install the bundle from `src-tauri/target/release/bundle/`. See the [development guide](DEVELOPMENT.md).

### I used Deezy. Will I lose my data when I install Wavekeep?

No. Wavekeep is Deezy's new name. The first time it starts, it moves your settings, download history and custom themes from Deezy's data folder into its own, and moves your saved Deezer ARL and Spotify login to Wavekeep's entries in the credential store. Your download folder setting and your music files are not touched.

With the `.exe` installer, Deezy stays installed next to Wavekeep. **Start Wavekeep once before you uninstall Deezy.** If you uninstall Deezy first and tick **"Delete the application data"**, its uninstaller deletes the data before Wavekeep can move it. The `.msi` installer upgrades Deezy in place, so this does not come up.

### What exactly does the move from Deezy do?

Every Tauri app has an internal identifier that decides where it keeps its data and how its credential-store entries are named. Deezy's was `com.pierr.deezy`; Wavekeep's is `com.pierr.wavekeep`. On every start, before anything is loaded, Wavekeep:

- **Moves your data:** `settings.json`, `download_history.json` and the `themes` folder go from the old folder to the new one (on Windows from `%APPDATA%\com.pierr.deezy` to `%APPDATA%\com.pierr.wavekeep`). Anything already in the new folder is kept and never overwritten. The old folder is removed only once it is empty.
- **Moves your logins:** each saved credential (Deezer ARL, Spotify tokens) is copied to the new name and read back to check it, and only then is the old entry deleted, so a secret is never lost. If Wavekeep already has a newer login, the old one is just removed. If an old entry cannot be removed, it stays and the log says so. With `WAVEKEEP_NO_KEYRING` set, the ARL is in `settings.json` and moves with it.

Once everything has moved these steps do nothing. What happened is in Wavekeep's log; see [Does Wavekeep keep a log?](#does-wavekeep-keep-a-log-what-is-in-it).

Good to know:

- After the move, the old Deezy app starts with default settings and no saved logins.
- On macOS, the Keychain may ask once whether Wavekeep may read the logins Deezy saved.
- On Windows, Deezy's browser cache in `%LOCALAPPDATA%\com.pierr.deezy` is not moved; you can delete that folder.
- On Linux, the old `~/.local/share/com.pierr.deezy` folder keeps Deezy's browser data, so it is not removed automatically; you can delete it.
- The startup warning appears once more, because that choice was stored with the browser data.
- The older `DEEZY_NO_KEYRING` and `DEEZY_SPOTIFY_CLIENT_ID` environment variables still work next to the `WAVEKEEP_` ones.

---

## ⚖️ Legal

### Is Wavekeep legal to use?

Wavekeep is provided for educational and personal use, but that does not make every use lawful. Downloading or extracting music can violate [Deezer's](https://www.deezer.com/legal/cgu) or [Spotify's](https://www.spotify.com/legal/end-user-agreement/) terms of service, copyright law, or both. A subscription does not give you permission to copy or share recordings.

Laws differ by country. You alone are responsible for finding out whether your use is allowed and for getting any permission you need. This FAQ is not legal advice. Do not redistribute downloaded music or use it commercially without the rights holders' permission.

### Does Wavekeep support the artists?

No. Downloading through Wavekeep does not pay artists or other rights holders. Support them through authorised streams, purchases, merchandise or live shows.

---

## 💬 Community & support

Question not answered here? Join the [Discord server](https://discord.gg/dvuWBeXSf3) or open an [issue on GitHub](https://github.com/PierrunoYT/Wavekeep/issues). Remember: Wavekeep is unofficial, comes without warranty, and is used at your own risk.
