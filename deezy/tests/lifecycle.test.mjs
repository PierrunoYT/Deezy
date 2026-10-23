import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, loadModule, settle, store } from './helpers.mjs';

test('closing Settings during initialization does not create orphan subscriptions', async () => {
  const pending = deferred();
  let cleanup;
  let subscriptions = 0;
  const observable = { subscribe() { subscriptions++; return () => subscriptions--; } };
  loadModule('../src/lib/components/SettingsView.svelte', {
    '@tauri-apps/api/core': { invoke: () => pending.promise },
    svelte: { onMount(fn) { cleanup = fn(); } },
    '$lib/stores': { theme: observable, currentLocale: observable, loggedIn: observable },
    '$lib/notifications': {}, 'svelte-i18n': {}, '$lib/i18n': {}, './ThemeManager.svelte': {}
  }, {
    $state: value => value, $derived: value => value, $props: () => ({}), $effect() {},
    $notificationsEnabled: false, $userInfo: null, $settingsArlDraft: '', $downloadDelaySeconds: 0,
    $maxConcurrentDownloads: 3
  });
  cleanup();
  pending.resolve({});
  await settle();
  assert.equal(subscriptions, 0);
});

test('changing the Spotify client ID saves a trimmed value and disconnects the old token', async () => {
  const calls = [];
  const spotifyConnected = store(true);
  const spotifyUserName = store('Listener');
  const { saveSpotifyClientId, setClientId } = loadModule('../src/lib/components/SettingsView.svelte', {
    '@tauri-apps/api/core': { invoke: async (command, args) => { calls.push({ command, args }); } },
    svelte: { onMount() {} },
    '$lib/stores': { spotifyConnected, spotifyUserName },
    '$lib/notifications': {}, 'svelte-i18n': {}, '$lib/i18n': {}, './ThemeManager.svelte': {}
  }, {
    $state: value => value, $derived: value => value, $props: () => ({}), $effect() {},
    $notificationsEnabled: false, $userInfo: null, $settingsArlDraft: '', $downloadDelaySeconds: 0,
    $maxConcurrentDownloads: 3, $spotifyConnected: true, $_: key => key
  }, `
    export { saveSpotifyClientId };
    export function setClientId(value, saved) {
      spotifyClientId = value;
      savedSpotifyClientId = saved;
    }
  `);

  setClientId('  not-a-client-id  ', 'old-client');
  await saveSpotifyClientId();
  assert.equal(calls.length, 0, 'an invalid Client ID is not sent to the backend');

  setClientId('  0123456789abcdef0123456789abcdef  ', 'old-client');
  await saveSpotifyClientId();

  assert.equal(calls.length, 2);
  assert.equal(calls[0].command, 'update_settings');
  assert.equal(calls[0].args.updates.spotify_client_id, '0123456789abcdef0123456789abcdef');
  assert.equal(calls[1].command, 'spotify_logout');
  assert.equal(spotifyConnected.value, false);
  assert.equal(spotifyUserName.value, null);

  await saveSpotifyClientId();
  assert.equal(calls.length, 2);
});

test('a rejected ARL is cleared from the field and the draft store', async () => {
  const settingsArlDraft = store('');
  const { saveSettings, setForm, inspect } = loadModule('../src/lib/components/SettingsView.svelte', {
    '@tauri-apps/api/core': { invoke: async command => {
      if (command === 'login') throw new Error('Invalid ARL');
    } },
    svelte: { onMount() {} },
    '$lib/stores': { settingsArlDraft },
    '$lib/notifications': {}, 'svelte-i18n': {}, '$lib/i18n': {}, './ThemeManager.svelte': {}
  }, {
    $state: value => value, $derived: value => value, $props: () => ({}), $effect() {},
    $notificationsEnabled: false, $userInfo: null, $settingsArlDraft: '', $downloadDelaySeconds: 0,
    $maxConcurrentDownloads: 3, $loggedIn: false, $spotifyConnected: false, $_: key => key
  }, `
    export { saveSettings };
    export function setForm(value) { arl = value; outputDir = 'C:/Music'; }
    export const inspect = () => ({ arl, statusMsg, statusType });
  `);

  const typed = 'a'.repeat(192);
  setForm(typed);
  settingsArlDraft.set(typed);
  await saveSettings();

  assert.equal(inspect().arl, '');
  assert.equal(settingsArlDraft.value, '');
  assert.equal(inspect().statusType, 'error');
  assert.equal(inspect().statusMsg, 'settings.status.loginFailed');
});

function traySetup() {
  const requests = [];
  let subscriptions = 0;
  let listeners = 0;
  const observable = { subscribe() { subscriptions++; return () => subscriptions--; } };
  const { trayManager } = loadModule('../src/lib/tray.ts', {
    '@tauri-apps/api/core': {},
    '@tauri-apps/api/event': { listen() {
      const request = deferred();
      requests.push(() => { listeners++; request.resolve(() => listeners--); });
      return request.promise;
    } },
    'svelte/store': {}, './downloadQueue': {},
    './stores': { activeDownloads: observable, downloadQueue: observable, pausedDownloads: observable }
  });
  return { trayManager, requests, inspect: () => ({ subscriptions, listeners }) };
}

test('concurrent tray initialization registers only one listener', async () => {
  const { trayManager, requests, inspect } = traySetup();
  const first = trayManager.init();
  const second = trayManager.init();
  assert.equal(requests.length, 1);
  requests[0]();
  await Promise.all([first, second]);
  assert.deepEqual(inspect(), { listeners: 1, subscriptions: 3 });
  trayManager.destroy();
  assert.deepEqual(inspect(), { listeners: 0, subscriptions: 0 });
});

test('tray teardown releases pending listeners without disrupting a new initialization', async () => {
  const { trayManager, requests, inspect } = traySetup();
  const old = trayManager.init();
  trayManager.destroy();
  const current = trayManager.init();
  requests[1]();
  await current;
  requests[0]();
  await old;
  assert.deepEqual(inspect(), { listeners: 1, subscriptions: 3 });
  trayManager.destroy();
  assert.deepEqual(inspect(), { listeners: 0, subscriptions: 0 });
});
