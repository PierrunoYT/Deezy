import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, loadModule, settle } from './helpers.mjs';

function searchSetup() {
  const pending = new Map();
  const api = loadModule('../src/lib/components/SearchView.svelte', {
    '@tauri-apps/api/core': { invoke(command, args) {
      const request = deferred(); pending.set(args.artistId ?? args.playlistId, request); return request.promise;
    } },
    '$lib/stores': {}, '$lib/downloadQueue': {}, '$lib/rateLimiter': {},
    svelte: { onMount() {} }, '$lib/keyboardShortcuts': {}, '$lib/audioPlayer': {},
    'svelte-i18n': {}, './search/SearchResults.svelte': {}, './search/SearchView.css': {}
  }, { $state: value => value, $effect() {}, $_: key => key }, `
    export { openArtist, closeArtist, openPlaylist, closePlaylist };
    export const inspect = () => ({ selectedArtist, artistAlbums, loadingDiscography, selectedPlaylist, playlistTracks, loadingPlaylist });
  `);
  return { api, pending };
}

test('older artist results cannot replace the current artist or finish its loading state', async () => {
  const { api, pending } = searchSetup();
  const first = api.openArtist(1, 'First', '');
  const second = api.openArtist(2, 'Second', '');
  pending.get('1').resolve([{ id: 10 }]);
  await first;
  assert.equal(api.inspect().artistAlbums.length, 0);
  assert.equal(api.inspect().loadingDiscography, true);
  pending.get('2').resolve([{ id: 20 }]);
  await second;
  assert.equal(api.inspect().artistAlbums[0].id, 20);
});

test('reopening the same playlist invalidates the previous request', async () => {
  const { api, pending } = searchSetup();
  const first = api.openPlaylist({ id: 1 });
  const oldRequest = pending.get('1');
  api.closePlaylist();
  const second = api.openPlaylist({ id: 1 });
  oldRequest.resolve([{ id: 10 }]);
  await first;
  assert.equal(api.inspect().playlistTracks.length, 0);
  assert.equal(api.inspect().loadingPlaylist, true);
  pending.get('1').resolve([{ id: 20 }]);
  await second;
  assert.equal(api.inspect().playlistTracks[0].id, 20);
});

function themeSetup() {
  const colors = new Map();
  const classes = new Set();
  const pending = deferred();
  const { applyTheme } = loadModule('../src/routes/+layout.svelte', {
    '../app.css': {}, svelte: { onMount() {}, onDestroy() {} },
    '@tauri-apps/api/core': { invoke: async command => command === 'get_settings' ? { custom_theme: 'custom' } : pending.promise },
    '@tauri-apps/api/event': {}, '$lib/stores': {}, '$lib/i18n': {}, 'svelte-i18n': {},
    '$lib/tray': {}, '$lib/downloadQueue': {}, '$lib/downloadHistory': {}, '$lib/notifications': {}
  }, {
    $state: value => value, $props: () => ({}),
    document: { documentElement: {
      style: { setProperty: (key, value) => colors.set(key, value), removeProperty: key => colors.delete(key) },
      classList: { remove: key => classes.delete(key), toggle: (key, enabled) => enabled ? classes.add(key) : classes.delete(key) }
    } },
    window: { matchMedia: () => ({ matches: false }) }
  }, 'export { applyTheme };');
  return { applyTheme, pending, colors, classes };
}

test('switching to a built-in theme removes custom inline colors', async () => {
  const { applyTheme, pending, colors, classes } = themeSetup();
  pending.resolve({ colors: { accent: '#123456' } });
  await applyTheme('custom');
  assert.equal(colors.get('--accent'), '#123456');
  await applyTheme('light');
  assert.equal(colors.size, 0);
  assert.equal(classes.has('light'), true);
});

test('a delayed custom theme cannot overwrite a newer built-in selection', async () => {
  const { applyTheme, pending, colors, classes } = themeSetup();
  const oldTheme = applyTheme('custom');
  await settle();
  await applyTheme('light');
  pending.resolve({ colors: { accent: '#123456' } });
  await oldTheme;
  assert.equal(colors.size, 0);
  assert.equal(classes.has('light'), true);
});
