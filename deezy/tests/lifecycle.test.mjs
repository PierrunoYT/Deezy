import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, loadModule, settle } from './helpers.mjs';

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
    $notificationsEnabled: false, $userInfo: null, $settingsArlDraft: ''
  });
  cleanup();
  pending.resolve({});
  await settle();
  assert.equal(subscriptions, 0);
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
