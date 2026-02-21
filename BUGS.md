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

## 🟠 High – Fixed

### 4. ~~XSS via `{@html}` on Deezer API data~~ ✅
- **File:** `deezy/src/lib/components/LyricsModal.svelte`
- **Fixed:** `formatLyrics()` now escapes `&`, `<`, `>`, `"`, and `'` before inserting `<br>` tags, preventing script injection from API data.

### 5. ~~Path traversal in theme operations~~ ✅
- **File:** `deezy/src-tauri/src/themes.rs`
- **Fixed:** Added `sanitize_theme_name()` that rejects names containing `.`, `/`, `\`, or `..`. Applied to `load_custom_theme`, `save_custom_theme`, and `delete_custom_theme`.

### 6. ~~Settings file has no file permissions restriction~~ ✅
- **File:** `deezy/src-tauri/src/settings.rs`
- **Fixed:** On Unix, settings file permissions are set to `0600` (owner read/write only) after writing. Windows uses ACL-based permissions which default to user-only access.

## 🟡 Medium – Fixed

### 7. ~~`withGlobalTauri` enabled~~ ✅
- **File:** `deezy/src-tauri/tauri.conf.json`
- **Fixed:** Set `withGlobalTauri` to `false`. The frontend uses `@tauri-apps/api` imports only.

### 8. `'unsafe-inline'` in CSP for styles ⏳ Deferred
- **File:** `deezy/src-tauri/tauri.conf.json:23`
- **Description:** `style-src 'self' 'unsafe-inline'` weakens CSP protection and allows injected inline styles.
- **Status:** Cannot remove — the app uses dynamic inline styles for progress bars, color swatches, and volume indicators. Would require migrating all dynamic styles to CSS classes or custom properties.

### 9. ~~Sensitive values in stderr logs~~ ✅
- **Files:** `deezy/src-tauri/src/deezer/mod.rs`, `deezy/src-tauri/src/commands.rs`
- **Fixed:** Removed all verbose `eprintln!` debug logging that exposed settings values, user IDs, session flow details, and API call traces.

### 10. ~~CSV injection in history export~~ ✅
- **File:** `deezy/src-tauri/src/commands.rs`
- **Fixed:** Added `sanitize_csv_field()` that prefixes values starting with `=`, `+`, `-`, `@`, `\t`, or `\r` with a single quote to prevent formula injection in spreadsheet applications.

## 🟢 Low – Fixed

### 11. ~~No TLS certificate pinning~~ ✅
- **File:** `deezy/src-tauri/src/deezer/mod.rs`
- **Fixed:** Enforced minimum TLS 1.2 and HTTPS-only connections on the reqwest client.

### 12. ~~Overly broad `process` capability~~ ✅
- **File:** `deezy/src-tauri/capabilities/default.json`
- **Fixed:** Removed `"process:default"`, keeping only `"process:allow-restart"`.
