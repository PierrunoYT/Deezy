import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, loadModule, settle } from './helpers.mjs';

function searchSetup() {
  const pending = new Map();
  const api = loadModule('../src/lib/components/SearchView.svelte', {
    '@tauri-apps/api/core': { invoke(command, args) {
      const request = deferred(); pending.set(args?.artistId ?? args?.playlistId ?? command, request); return request.promise;
    } },
    '$lib/stores': {}, '$lib/downloadQueue': {}, '$lib/rateLimiter': {},
    svelte: { onMount() {} }, '$lib/keyboardShortcuts': {}, '$lib/audioPlayer': {},
    'svelte-i18n': {}, './search/SearchResults.svelte': {}, './search/SearchView.css': {},
    './search/types': loadModule('../src/lib/components/search/types.ts', {})
  }, { $state: value => value, $derived: value => value, $effect() {}, $_: key => key }, `
    export { openArtist, closeArtist, openPlaylist, closePlaylist, switchSearchType };
    export const login = () => { isLoggedIn = true; };
    export const inspect = () => ({ selectedArtist, artistAlbums, loadingDiscography, selectedPlaylist, playlistTracks, loadingPlaylist, results, playlistResults, searching, errorMsg });
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

test('a library tab that finishes loading after another tab opened cannot replace its results', async () => {
  const { api, pending } = searchSetup();
  api.login();
  api.switchSearchType('loved');
  const lovedRequest = pending.get('get_loved_tracks');
  api.switchSearchType('myPlaylists');
  lovedRequest.resolve([{ id: 10 }]);
  await settle();
  assert.equal(api.inspect().results.length, 0);
  assert.equal(api.inspect().searching, true);
  pending.get('get_user_playlists').resolve([]);
  await settle();
  assert.equal(api.inspect().searching, false);
  assert.equal(api.inspect().errorMsg, 'search.library.noPlaylists');
});

function themeSetup() {
  const colors = new Map();
  const classes = new Set();
  const pending = deferred();
  const { applyTheme } = loadModule('../src/routes/+layout.svelte', {
    '../app.css': {}, svelte: { onMount() {}, onDestroy() {} },
    '@tauri-apps/api/core': { invoke: async command => command === 'get_settings' ? { custom_theme: 'custom' } : pending.promise },
    '@tauri-apps/api/event': {}, '$lib/stores': {}, '$lib/i18n': {}, 'svelte-i18n': {},
    '$lib/tray': {}, '$lib/downloadQueue': {}, '$lib/downloadHistory': {}, '$lib/notifications': {},
    '$lib/logger': {}, '$lib/themeColors': loadModule('../src/lib/themeColors.ts', {})
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

test('custom theme values that are not plain colors are not applied', async () => {
  const { applyTheme, pending, colors } = themeSetup();
  pending.resolve({ colors: {
    accent: '#000;position:fixed;inset:0',
    'bg-dark': '#000 url(https://example.invalid/x.png)',
    bg_surface: 'rgb(1, 2, 3)',
    success: 'var(--x, red)'
  } });
  await applyTheme('custom');
  assert.equal(colors.has('--accent'), false);
  assert.equal(colors.has('--bg-dark'), false);
  assert.equal(colors.has('--success'), false);
  assert.equal(colors.get('--bg-surface'), 'rgb(1, 2, 3)');
});

test('theme colors are checked with CSS.supports when the browser provides it', () => {
  const calls = [];
  const { isSafeThemeColor } = loadModule('../src/lib/themeColors.ts', {}, {
    CSS: { supports(property, value) { calls.push(property); return value !== 'notacolor'; } }
  });
  assert.equal(isSafeThemeColor('notacolor'), false);
  assert.equal(isSafeThemeColor('rebeccapurple'), true);
  assert.equal(isSafeThemeColor('#000; color: red'), false);
  assert.equal(isSafeThemeColor(42), false);
  assert.deepEqual(calls, ['color', 'color']);
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
