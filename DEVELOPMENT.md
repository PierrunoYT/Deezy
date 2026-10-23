# Developing Wavekeep

Wavekeep uses Tauri v2 and Rust for the desktop backend, with a Svelte 5/SvelteKit
frontend in SPA mode. Server-side rendering is disabled in
`deezy/src/routes/+layout.ts`.

## Setup and commands

Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for
your platform, stable Rust, Bun, and Node.js with npm. From the repository root,
enter the application directory and install dependencies:

```bash
cd deezy
bun install --frozen-lockfile
bun run tauri dev
```

Run the remaining application commands from `deezy/`.

The committed `bun.lock` defines frontend dependencies. Reinstall with
`--frozen-lockfile` after pulling dependency updates; stale `node_modules` can
otherwise make local checks use different versions. The `test` package script
uses Node's test runner even when invoked through Bun.

| Command | Purpose |
| --- | --- |
| `npm test` | Frontend regression tests |
| `npm run check` | Svelte/TypeScript diagnostics |
| `npm run build` | Production frontend build |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust compilation and tests |
| `bun run tauri dev` | Vite server and native desktop window |
| `bun run tauri build` | Native production build and configured installers |

`npm run build` alone does not build a native installer.

## Module ownership

Paths below are relative to `deezy/`.

| Path | Responsibility |
| --- | --- |
| `src/routes/+layout.svelte` | Startup, theme/i18n initialization, progress events, history persistence, exit handling |
| `src/routes/+page.svelte` | Application shell and view selection |
| `src/lib/stores.ts` | Shared types and Svelte stores |
| `src/lib/downloadQueue.ts` | Queue, concurrency, cancellation, resume, and history clearing |
| `src/lib/downloadHistory.ts` | Recover interrupted history entries as paused |
| `src/lib/audioPlayer.ts` | Preview playback and playback-request lifetime |
| `src/lib/tray.ts` | Tray event listener and status subscriptions |
| `src/lib/components/search/` | Search and library (Loved Tracks, My Playlists) result rendering, types, and styles |
| `src-tauri/src/lib.rs` | Managed application state, command registration, window events |
| `src-tauri/src/commands/` | IPC handlers grouped by account, catalog, downloads, filesystem, history, settings, Spotify, tag editor, themes, and tray |
| `src-tauri/src/deezer/` | Authentication, gateway and catalog requests, user library (Loved Tracks, profile playlists), media resolution, crypto, downloads, and models |
| `src-tauri/src/spotify/` | OAuth/AP sessions, Web API and Pathfinder search, AP catalog metadata, librespot audio streaming, and OGG/FLAC/MP3/M4A tagging |
| `src-tauri/src/paths.rs` | Provider-neutral destination paths and safe temp-file finalization shared by both engines |
| `src-tauri/src/settings.rs` | Validated settings, credential storage, atomic private-file writes |
| `src-tauri/src/themes.rs` | Custom theme validation and persistence |

`commands.rs` and `deezer/mod.rs` contain shared helpers and module exports;
handlers and client methods are split into their respective submodules.

## Download and persistence behavior

`DownloadQueueManager` owns the active count and limits concurrency to the
simultaneous downloads setting (1–10, default 3, read from the
`maxConcurrentDownloads` store on every pass, so changes apply right away). UI progress events update statuses without changing that count.
Notification delivery does not hold a download slot. When the pause-between-
downloads setting is set, starts are spaced at least that far apart (read from
the `downloadDelaySeconds` store); a skipped track releases its spacing.

The backend resolves metadata and media, streams/decrypts audio into a unique
`.wavekeep.part` file, and writes tags on that temporary file. Tagging failures emit
warnings without failing the audio download. Final publication avoids overwriting
existing files, choosing a numbered name on collision. Filesystems without hard
link support use an atomic no-overwrite rename instead.

With skip existing files enabled (the default), the backend first builds the
destination path and checks for a non-empty supported audio file. Deezer checks
`.mp3` and `.flac`; Spotify checks `.ogg`, `.flac`, `.mp3`, and `.m4a`. A match
returns `status: "skipped"` without opening an audio stream. Only the current
folder structure is checked; the download folder is not scanned.

A download speed limit paces received bytes through one `BandwidthLimiter` in
`AppState`, so the limit applies to all concurrent downloads combined.

Pause requests cancel the transfer. Resume starts a fresh download, not a ranged
continuation. Completion is final once the file is published; later pause requests
must not remove or mark that file paused.

History snapshots normalize resolving, downloading, and tagging rows to paused
for recovery, without changing the live rows. Writes are debounced and serialized
in the frontend and use atomic file replacement in the backend. A normal exit
cancels active work and flushes pending history. A forced termination can lose
the most recent unsaved updates. Pending queue entries are not persisted.

Clearing history retains active and queued rows and never deletes audio files.
Deezer catalog list loaders follow pagination, validate API hosts, upgrade legacy
HTTP pagination links to HTTPS, and reject loops, exceeded limits, and failed
later pages rather than returning a silently incomplete list.

## Spotify catalog and authentication

Spotify separates catalog and download responsibilities:

- `api.rs` handles Web API search and user playlists. Track, album, and artist
  searches fall back to `pathfinder.rs` only after a Web API rate-limit error
  (Spotify currently answers that fallback with `403 Client/request not
  allowed`); playlist search remains on the Web API. Short 429 waits are retried at most
  twice, while `Retry-After` values over five seconds fail immediately.
- `ap.rs` resolves track, album, artist, playlist, Loved Tracks, and download
  metadata through the librespot AP session, with a Web API fallback for artist
  discographies when AP metadata is rejected. Track metadata uses eight concurrent
  requests while preserving result order. Artist discographies are capped at 120
  albums. Unavailable individual tracks are skipped, but a completely failed
  track load returns an error.
- `download.rs` selects the best available audio format and streams through the
  AP session. `tags.rs` writes the provider-appropriate OGG, FLAC, MP3, or M4A tags.

OAuth uses `http://127.0.0.1:5588/login`. The AP session (track metadata and
audio) always logs in with librespot's shared default Client ID, because Spotify
rejects AP metadata requests made under any other ID with a 400. The Web API
(search, playlists, library) uses the Client ID resolved from the saved
`spotify_client_id` setting, then `WAVEKEEP_SPOTIFY_CLIENT_ID` (or the older
`WAVEKEEP_SPOTIFY_CLIENT_ID`), then the default;
with a custom ID this means a second browser login. Tokens are bound to the
Client ID that issued them, so changing the setting while connected logs out
both. The default token uses the `com.pierr.deezy` / `spotify_oauth`
credential-store entry and the custom-ID token uses `spotify_oauth_web`;
persistence is best-effort and a keyring failure requires a fresh login after
restart. Because both logins reuse the callback port, the success page carries
an empty favicon and the second login waits briefly and retries on a stray
request.

The Spotify account type (`Profile.product`) is displayed but never enforced,
and Free-account audio access through librespot has not been verified. Available
quality depends on the formats Spotify lists for each track.

`spotify::debug_log` writes to stderr and to `wavekeep-debug.log` in the temp
directory, and `lib.rs` logs panics through it. It also records app start,
every `download_track` command result (both providers), each Spotify download
step, Pathfinder rejections, and the query and top results of each Web API
search. Use that file to diagnose Spotify download and search failures,
especially in builds without a console.

## Implementation conventions

- Use Svelte 5 runes (`$state`, `$derived`, `$props`) for component reactivity.
- Dispose subscriptions/listeners even when registration finishes after unmount.
  Guard asynchronous results so older requests cannot replace newer selections.
- Register new Rust IPC handlers in `lib.rs`. Follow the existing command naming
  and camelCase frontend parameter conventions; return `Result<T, String>`.
- `get_settings` redacts the ARL. Keep credential persistence in Rust and preserve
  stored credentials during non-authentication settings updates. Credential-store
  fallback is shown explicitly in Settings; avoid logging credentials or gateway
  URLs containing session tokens.
- Use backend settings patches for individual preferences. Settings I/O and
  account replacement share the settings I/O lock.
- Keep the free-account MP3 128 quality override in both initial and retry paths.
- Expand folder-template placeholders once per template segment, then sanitize
  metadata. Metadata separators must not introduce additional directories.
- Add UI translation keys to all six files in `src/lib/i18n/locales/`.
- Check `src-tauri/tauri.conf.json` when introducing image or media origins;
  renderer resources are constrained by the Content Security Policy.

## Testing

Frontend tests live in `tests/*.test.mjs`. The helper transpiles TypeScript and
component script blocks, then supplies mocked stores, browser APIs, and IPC. The
tests cover state transitions and asynchronous races, not rendered Svelte DOM
behavior. Rust tests live beside the backend code under `#[cfg(test)]`.

The [September code review](deezy/CODE_REVIEW.md) records the validation as of
2026-09-10: 17 frontend tests, 15 Rust tests (the suites have grown since, and the
Spotify code postdates that review), clean Svelte diagnostics, and a production
frontend build. Live authenticated downloads, native notification prompts,
installer generation, and macOS/Linux runtime behavior were not exercised in
that review. Test these separately when changes affect those paths.

## Release checklist

1. Pick the version number with the semantic-versioning rules in `AGENTS.md`
   (patch for fixes only, minor for any new feature). Update it together in `deezy/package.json`,
   `deezy/src-tauri/Cargo.toml`, and `deezy/src-tauri/tauri.conf.json`. The committed
   `bun.lock` does not contain the app version, so a bump alone needs no lockfile
   change (only a dependency change does). `Cargo.lock` is currently ignored by
   this repository, so Rust dependencies are not pinned in Git.
2. Move the relevant `CHANGELOG.md` Unreleased entries into a dated version
   section. Keep source-only changes distinct from shipped installer behavior.
3. Install locked dependencies and run the test, check, and frontend build
   commands above.
4. Smoke-test Deezer and Spotify login/session restoration, search and rate-limit
   fallback, catalog loading, previews where available, download/pause/resume,
   tagging, restart recovery, themes, and tray shutdown on the release platform.
5. Build with `bun run tauri build` and verify the generated artifacts under
   `src-tauri/target/release/bundle/`.
6. Until Deezy installs are gone, the release notes must say that the `.exe`
   installer installs Wavekeep next to Deezy, and that Deezy should be uninstalled
   without ticking "Delete the application data", because both apps share the
   `com.pierr.deezy` data folder. See `AGENTS.md` for why the identifier is kept.
7. Make one release commit named `chore: prepare version X.Y.Z release` and an
   annotated tag `vX.Y.Z` with the message `Wavekeep vX.Y.Z`, then publish the
   appropriate installers and notes through the repository's release process.
   Do not push the commit or tag before the smoke test in step 4 has passed. The
   app has no automatic updater.
