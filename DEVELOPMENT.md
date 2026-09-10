# Developing Deezy

Deezy uses Tauri v2 and Rust for the desktop backend, with a Svelte 5/SvelteKit
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
| `src/lib/components/search/` | Search result rendering, types, and styles |
| `src-tauri/src/lib.rs` | Managed application state, command registration, window events |
| `src-tauri/src/commands/` | IPC handlers grouped by account, catalog, downloads, filesystem, history, settings, tags, themes, and tray |
| `src-tauri/src/deezer/` | Authentication, gateway and catalog requests, media resolution, crypto, downloads, and models |
| `src-tauri/src/settings.rs` | Validated settings, credential storage, atomic private-file writes |
| `src-tauri/src/themes.rs` | Custom theme validation and persistence |

`commands.rs` and `deezer/mod.rs` contain shared helpers and module exports;
handlers and client methods are split into their respective submodules.

## Download and persistence behavior

`DownloadQueueManager` owns the active count and limits concurrency to three
downloads. UI progress events update statuses without changing that count.
Notification delivery does not hold a download slot.

The backend resolves metadata and media, streams/decrypts audio into a unique
`.deezy.part` file, and writes tags on that temporary file. Tagging failures emit
warnings without failing the audio download. Final publication avoids overwriting
existing files, choosing a numbered name on collision. Filesystems without hard
link support use a copy fallback with partial-file cleanup.

Pause requests cancel the transfer. Resume starts a fresh download, not a ranged
continuation. Completion is final once the file is published; later pause requests
must not remove or mark that file paused.

History snapshots normalize resolving, downloading, and tagging rows to paused
for recovery, without changing the live rows. Writes are debounced and serialized
in the frontend and use atomic file replacement in the backend. A normal exit
cancels active work and flushes pending history. A forced termination can lose
the most recent unsaved updates. Pending queue entries are not persisted.

Clearing history retains active and queued rows and never deletes audio files.
Catalog list loaders follow pagination, validate API hosts, upgrade legacy HTTP
pagination links to HTTPS, and reject loops, exceeded limits, and failed later
pages rather than returning a silently incomplete list.

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

The [September code review](deezy/CODE_REVIEW.md) records the completed validation:
17 frontend tests, 15 Rust tests, clean Svelte diagnostics, and a production
frontend build. Live authenticated downloads, native notification prompts,
installer generation, and macOS/Linux runtime behavior were not exercised in
that review. Test these separately when changes affect those paths.

## Release checklist

1. Update the version together in `deezy/package.json`,
   `deezy/src-tauri/Cargo.toml`, and `deezy/src-tauri/tauri.conf.json`. Refresh any
   affected metadata in the committed frontend lockfile. `Cargo.lock` is currently
   ignored by this repository.
2. Move the relevant `CHANGELOG.md` Unreleased entries into a dated version
   section. Keep source-only changes distinct from shipped installer behavior.
3. Install locked dependencies and run the test, check, and frontend build
   commands above.
4. Smoke-test login, search/pagination, previews, download/pause/resume, tagging,
   restart recovery, themes, and tray shutdown on the release platform.
5. Build with `bun run tauri build` and verify the generated artifacts under
   `src-tauri/target/release/bundle/`.
6. Create the release commit/tag and publish the appropriate installers and notes
   through the repository's release process. The app has no automatic updater.
