# Code review — 2026-09-10

Reviewed download queue and history handling, preview playback, themes, search
and catalog loading, backend authentication, HTTP handling, filename generation,
persistence, and component/tray cleanup. Fixed confirmed issues in five batches.

## Fixes

1. **Preview and themes:** invalidate obsolete playback failures; reject non-finite
   media control values; unload stopped media without assigning an empty URL;
   remove custom CSS overrides when selecting a built-in theme; ignore stale
   asynchronous theme loads; apply theme changes after successful persistence.
2. **Download state:** prevent delayed pause responses from overwriting completion;
   restore state after cancellation IPC failures; preserve active/queued rows when
   clearing history; recover interrupted phases as paused; restore status stores
   from history; remove the duplicate download-progress listener.
3. **Backend:** handle Unicode release dates without byte-slicing panics; preserve
   literal metadata slashes/placeholders in folder templates; sanitize Windows
   device names and control characters; reject overflowing numeric tags; save
   history through the existing atomic private-file writer; reject obsolete
   auto-login results; reject HTTP error bodies; omit request URLs from gateway
   errors; bound redirect chains.
4. **Catalogs and search:** follow pagination for albums, artist discographies,
   and playlists; enforce API-host and page/entry limits; reject pagination loops
   and partial results after failures; ignore stale detail responses; cancel
   pending search debounce timers when superseded.
5. **Lifecycle and completion:** prevent late Settings and tray initialization
   from leaking subscriptions; deduplicate concurrent tray initialization; clean
   up layout listeners registered after teardown; release download slots without
   waiting for notification permission; keep completed downloads unpausable.

## Validation

Run commands from `deezy/`:

- `bun install --frozen-lockfile` — synchronized stale local dependencies with the
  existing lockfile; no dependency versions were changed in the repository.
- `npm test` — 17 frontend regression tests.
- `npm run check` — no Svelte/TypeScript errors or warnings.
- `npm run build` — production frontend build.
- `cargo test --manifest-path src-tauri/Cargo.toml` — 15 Rust tests.
- `git diff --check` — whitespace validation.

Frontend regression tests exercise TypeScript logic with mocked browser/IPC
dependencies; they do not replace a rendered UI test. Live authenticated Deezer
downloads, OS notification prompts, installer generation, and macOS/Linux runtime
behavior were not exercised. This review does not establish that every possible
defect has been eliminated.
