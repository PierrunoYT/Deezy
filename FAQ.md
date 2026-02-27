# Deezy – Frequently Asked Questions

---

## 🔒 Security & Privacy

### Is Deezy stealing my ARL token?

No. Here's exactly what happens with your ARL:

- **Stored locally** — Your ARL is saved in your OS credential store (Windows Credential Manager / macOS Keychain / Linux Secret Service), the same place your browser saves passwords. It is never written to a plaintext file.
- **Never sent anywhere except Deezer** — The ARL is only used to authenticate directly with `deezer.com`. There is no backend server, no telemetry, and no third-party endpoint that ever receives it.
- **Never exposed to the UI** — The ARL is redacted before it's ever sent to the frontend. Only the Rust backend handles the raw token.
- **HTTPS-only** — All connections enforce TLS 1.2+ so traffic can't be intercepted in transit.
- **Fully open source** — Every line of code is public on GitHub. You can audit exactly what the app does with your token:
  - Token storage → `src-tauri/src/settings.rs`
  - API calls → `src-tauri/src/deezer/mod.rs`
  - Frontend redaction → `src-tauri/src/commands.rs`

### Does Deezy collect any data or analytics?

No. Deezy has no backend server, no analytics, no crash reporting, and no telemetry of any kind. Everything runs locally on your machine.

### Is it safe to use my Deezer account?

You use your own ARL token from your own browser session. Deezy never asks for your Deezer username or password. The risk profile is the same as being logged into Deezer in your browser.

---

## 🎵 Downloads & Quality

### What quality can I download in?

| Account Type | Available Qualities |
|---|---|
| Free | MP3 128 kbps |
| Premium | MP3 128 kbps, MP3 320 kbps, FLAC |

### Why did my download fall back to a lower quality?

Not all tracks are available in every quality on Deezer's servers. Deezy automatically falls back through the chain: **FLAC → MP3 320 → MP3 128**. The download history shows both the requested and actual quality when a fallback occurs.

### Where are my downloaded files saved?

In the folder you chose in Settings. You can also configure the folder structure (Flat, Artist/Track, Artist/Album/Track, or Album/Track).

### Can I download entire albums or playlists?

Yes. Click **Download All** on any album or playlist to queue all tracks at once.

---

## 🛠️ Setup & Troubleshooting

### How do I get my ARL token?

1. Log into [deezer.com](https://www.deezer.com) in your browser
2. Open DevTools (`F12`) → **Application** (Chrome) or **Storage** (Firefox) → **Cookies** → `https://www.deezer.com`
3. Copy the value of the `arl` cookie (192-character string)
4. Paste it into Deezy's Settings and click **Save & Login**

### My ARL token expired. What do I do?

ARL tokens expire periodically. Just log into Deezer in your browser, grab the new `arl` cookie value, and paste it into Deezy's Settings again.

### The app shows a blank/black screen on startup. What do I do?

This is usually a first-launch timing issue. Try closing and reopening the app. If it persists, delete the app data folder and re-enter your settings:
- **Windows:** `%APPDATA%\com.deezy.app`
- **macOS:** `~/Library/Application Support/com.deezy.app`
- **Linux:** `~/.local/share/com.deezy.app`

### Downloads are stalling or not starting. What do I do?

- Check that your ARL token is still valid (try logging into deezer.com)
- Check your internet connection
- Restart the app — the download queue state is restored on relaunch
- If a specific track keeps failing, it may not be available in the requested quality or region

### Album covers or audio previews don't load in the installed app but work in dev mode.

This was a known CSP issue fixed in **v0.2.8**. Make sure you're on the latest version.

---

## ⚖️ Legal

### Is Deezy legal to use?

Deezy is provided for **educational and personal use only**. Downloading music from Deezer may violate their [Terms of Service](https://www.deezer.com/legal/cgu). You are responsible for how you use the software and for complying with copyright laws in your jurisdiction. Downloaded content should not be redistributed or used commercially.

### Does Deezy support the artists?

Using Deezy does not generate royalty payments to artists. If you enjoy an artist's music, consider streaming it on Deezer, buying their music, or attending their shows.

---

## 💬 Community & Support

Have a question not answered here? Join the [Discord server](https://discord.gg/dvuWBeXSf3) or open an [issue on GitHub](https://github.com/PierrunoYT/Deezy/issues).
