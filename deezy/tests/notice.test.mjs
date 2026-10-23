import test from 'node:test';
import assert from 'node:assert/strict';
import { loadModule } from './helpers.mjs';

function setup(initial = {}, overrides = {}) {
  const data = { ...initial };
  const localStorage = {
    getItem: key => (key in data ? data[key] : null),
    setItem: (key, value) => { data[key] = String(value); },
    ...overrides
  };
  const api = loadModule('../src/lib/notice.ts', {}, { globalThis: { localStorage } });
  return { api, data };
}

test('the notice shows on a fresh install', () => {
  const { api } = setup();
  assert.equal(api.shouldShowStartupNotice(), true);
});

test('dismissing without "don\'t show again" does not hide it next time', () => {
  const { api, data } = setup();
  api.dismissStartupNotice(false);
  assert.deepEqual(data, {});
  assert.equal(api.shouldShowStartupNotice(), true);
});

test('"don\'t show again" hides it, and a newer notice version brings it back', () => {
  const { api, data } = setup();
  api.dismissStartupNotice(true);
  assert.equal(api.shouldShowStartupNotice(), false);

  const stored = Object.keys(data)[0];
  const older = setup({ [stored]: String(api.NOTICE_VERSION - 1) });
  assert.equal(older.api.shouldShowStartupNotice(), true);
});

test('missing or throwing storage never breaks the app', () => {
  const missing = loadModule('../src/lib/notice.ts', {}, { globalThis: {} });
  assert.equal(missing.shouldShowStartupNotice(), true);
  assert.doesNotThrow(() => missing.dismissStartupNotice(true));

  const throwing = setup({}, {
    getItem() { throw new Error('denied'); },
    setItem() { throw new Error('denied'); }
  });
  assert.equal(throwing.api.shouldShowStartupNotice(), true);
  assert.doesNotThrow(() => throwing.api.dismissStartupNotice(true));
});

test('every listed item has an English text', async () => {
  const { readFileSync } = await import('node:fs');
  const en = JSON.parse(readFileSync(new URL('../src/lib/i18n/locales/en.json', import.meta.url), 'utf8'));
  const { api } = setup();
  for (const id of api.IMPORTANT_ITEMS) {
    assert.ok(en.info.items[id]?.title && en.info.items[id]?.text, `info.items.${id}`);
  }
  for (const id of api.FAQ_ITEMS) {
    assert.ok(en.info.faq[id]?.q && en.info.faq[id]?.a, `info.faq.${id}`);
  }
});
