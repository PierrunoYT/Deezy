# Deezy – Security Bugs

## 🔴 Critical – Fixed

### 1. ~~ARL token stored in plaintext on disk~~ ✅
- **File:** `deezy/src-tauri/src/settings.rs`
- **Fixed:** ARL is now stored in the OS credential store (Windows Credential Manager / macOS Keychain / Linux Secret Service) via the `keyring` crate. The JSON settings file no longer contains the ARL. Existing plaintext ARLs are automatically migrated to the keyring on first load. Falls back to file storage if keyring is unavailable.

### 2. ~~Hardcoded cryptographic keys~~ ✅
- **File:** `deezy/src-tauri/src/deezer/crypto.rs`
- **Fixed:** Blowfish and AES-128 keys are stored as XOR-obfuscated byte arrays and deobfuscated at runtime. Keys no longer appear as readable string literals in the binary.

### 3. ~~Auto-updater public key is a placeholder~~ ✅
- **File:** `deezy/src-tauri/tauri.conf.json`
- **Fixed:** Generated a real Ed25519/minisign keypair. Public key set in config; private key stored in `src-tauri/keys/updater.key` (gitignored). Set `TAURI_SIGNING_PRIVATE_KEY_PATH` in CI to sign releases.

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
