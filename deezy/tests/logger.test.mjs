import test from 'node:test';
import assert from 'node:assert/strict';
import { loadModule, settle } from './helpers.mjs';

function setup(invokeImpl) {
  const calls = [];
  const api = loadModule('../src/lib/logger.ts', {
    '@tauri-apps/api/core': {
      invoke: async (command, args) => {
        calls.push({ command, args });
        if (invokeImpl) return invokeImpl();
      }
    }
  }, { Error, JSON });
  return { api, calls };
}

function fakeWindow() {
  const listeners = new Map();
  const seen = [];
  return {
    seen,
    listeners,
    target: {
      console: { error: (...args) => seen.push(['error', ...args]), warn: (...args) => seen.push(['warn', ...args]) },
      addEventListener: (name, fn) => listeners.set(name, fn),
      removeEventListener: name => listeners.delete(name)
    }
  };
}

test('log sends one write_log call with a readable message', async () => {
  const { api, calls } = setup();
  api.log('warn', 'download', 'Failed:', new Error('boom'), { id: 3 });
  await settle();
  assert.equal(calls.length, 1);
  assert.equal(calls[0].command, 'write_log');
  assert.equal(calls[0].args.level, 'warn');
  assert.equal(calls[0].args.area, 'download');
  assert.match(calls[0].args.message, /^Failed: Error: boom.* \{"id":3\}$/);
});

test('a failing backend never throws or recurses', async () => {
  const { api } = setup(() => Promise.reject(new Error('no backend')));
  assert.doesNotThrow(() => api.log('error', 'x', 'y'));
  await settle();
});

test('console errors and warnings still print and are also logged; uninstall restores them', async () => {
  const { api, calls } = setup();
  const win = fakeWindow();
  const originalError = win.target.console.error;
  const uninstall = api.installErrorLogging(win.target);

  win.target.console.error('Download failed:', 'timeout');
  win.target.console.warn('slow');
  win.listeners.get('unhandledrejection')({ reason: new Error('lost') });
  await settle();

  assert.deepEqual(win.seen, [['error', 'Download failed:', 'timeout'], ['warn', 'slow']]);
  assert.deepEqual(calls.map(c => [c.args.level, c.args.area]), [['error', 'console'], ['warn', 'console'], ['error', 'uncaught']]);
  assert.equal(calls[0].args.message, 'Download failed: timeout');

  uninstall();
  assert.equal(win.target.console.error, originalError);
  assert.equal(win.listeners.size, 0);
});
