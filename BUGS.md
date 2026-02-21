# Deezy – Security Bugs

## 🔴 Critical

### 1. ARL token stored in plaintext on disk
- **File:** `deezy/src-tauri/src/settings.rs:136`
- **Description:** The Deezer ARL authentication cookie is saved as plain JSON in `settings.json`. Any process or malware on the machine can read it.
- **Fix:** Use the OS credential store (e.g. `tauri-plugin-stronghold` or Windows Credential Manager) instead of writing secrets to a plain JSON file.

### 2. Hardcoded cryptographic keys
- **File:** `deezy/src-tauri/src/deezer/crypto.rs:9, 40`
- **Description:** Blowfish secret (`g4el58wc0zvf9na1`) and AES-128 key (`jo6aey6haid2Teih`) are embedded as string literals in source code.
- **Fix:** Obfuscate or load keys at runtime to reduce exposure.

### 3. Auto-updater public key is a placeholder
- **File:** `deezy/src-tauri/tauri.conf.json:42`
- **Description:** `"pubkey": "REPLACE_WITH_YOUR_PUBLIC_KEY"` means the updater cannot verify update signatures, allowing malicious update injection.
- **Fix:** Generate a real Ed25519 keypair and set the public key before shipping.

## 🟠 High

### 4. XSS via `{@html}` on Deezer API data
- **File:** `deezy/src/lib/components/LyricsModal.svelte:152`
- **Description:** `formatLyrics()` replaces `\n` with `<br>` and renders with `{@html}`. If Deezer returns lyrics containing `<script>` tags or event handler attributes, they execute in the webview.
- **Fix:** Escape `<`, `>`, `&`, `"`, and `'` before inserting `<br>` tags.

### 5. Path traversal in theme operations
- **Files:** `deezy/src-tauri/src/themes.rs:118, 134, 143`
- **Description:** `load_custom_theme`, `save_custom_theme`, and `delete_custom_theme` build file paths from user-supplied `theme_name` without sanitizing `..` or path separators. A crafted name like `../../settings` could read or overwrite arbitrary files in the app data directory.
- **Fix:** Validate that the filename contains no path separators (`/`, `\`) or `..` sequences.

### 6. Settings file has no file permissions restriction
- **File:** `deezy/src-tauri/src/settings.rs:136`
- **Description:** `std::fs::write` uses default OS permissions. On some systems the file may be world-readable, exposing the ARL token.
- **Fix:** Set restrictive file permissions (e.g. `0600` on Unix) when writing `settings.json`.

## 🟡 Medium

### 7. `withGlobalTauri` enabled
- **File:** `deezy/src-tauri/tauri.conf.json:13`
- **Description:** `"withGlobalTauri": true` exposes the Tauri API on `window.__TAURI__`. If any XSS occurs (see #4), the attacker gains full access to all registered Tauri commands.
- **Fix:** Set to `false` and rely on `@tauri-apps/api` imports only.

### 8. `'unsafe-inline'` in CSP for styles
- **File:** `deezy/src-tauri/tauri.conf.json:23`
- **Description:** `style-src 'self' 'unsafe-inline'` weakens CSP protection and allows injected inline styles.
- **Fix:** Remove `'unsafe-inline'` if possible, or use nonces/hashes for required inline styles.

### 9. Sensitive values in stderr logs
- **Files:** `deezy/src-tauri/src/deezer/mod.rs:60`, `deezy/src-tauri/src/commands.rs:128`
- **Description:** Debug `eprintln!` calls log settings values (output directory, quality) and login flow details. In debug builds these could inadvertently include or be adjacent to ARL values.
- **Fix:** Audit all `eprintln!` calls to ensure no auth tokens or session values leak.

### 10. CSV injection in history export
- **File:** `deezy/src-tauri/src/commands.rs:364-376`
- **Description:** CSV field values from track metadata (title, artist) are only double-quote-escaped. A track title starting with `=`, `+`, `-`, or `@` could trigger formula injection when opened in Excel.
- **Fix:** Prefix cell values that start with `=`, `+`, `-`, `@`, `\t`, or `\r` with a single quote (`'`).

## 🟢 Low – Fixed

### 11. ~~No TLS certificate pinning~~ ✅
- **File:** `deezy/src-tauri/src/deezer/mod.rs`
- **Fixed:** Enforced minimum TLS 1.2 and HTTPS-only connections on the reqwest client.

### 12. ~~Overly broad `process` capability~~ ✅
- **File:** `deezy/src-tauri/capabilities/default.json`
- **Fixed:** Removed `"process:default"`, keeping only `"process:allow-restart"`.
