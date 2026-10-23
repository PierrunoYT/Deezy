import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, loadModule } from './helpers.mjs';

function tagEditorSetup(invoke) {
  const effects = [];
  const api = loadModule('../src/lib/components/TagEditorModal.svelte', {
    '@tauri-apps/api/core': { invoke },
    svelte: { onMount() {}, untrack: fn => fn() },
    'svelte-i18n': {}
  }, {
    $state: value => value, $props: () => ({ show: true, initialFilePath: null, onClose() {} }),
    $effect(fn) { effects.push(fn); }, $_: key => key
  }, `
    export { loadTagsFromPath, pickFile, pickCover };
    export function setShow(value) { show = value; }
    export const inspect = () => ({ tags, title, error, loading, coverPreviewUrl });
  `);
  api.close = () => { api.setShow(false); effects.forEach(fn => fn()); };
  return api;
}

test('a tag read that finishes after the editor closed does not fill it again', async () => {
  const pending = deferred();
  const api = tagEditorSetup(() => pending.promise);

  const load = api.loadTagsFromPath('C:/Music/a.mp3');
  assert.equal(api.inspect().loading, true);
  api.close();
  pending.resolve({ file_path: 'C:/Music/a.mp3', format: 'mp3', title: 'Late' });
  assert.equal(await load, false);
  assert.equal(api.inspect().tags, null);
  assert.equal(api.inspect().title, '');
  assert.equal(api.inspect().loading, false);
});

test('only the newest of two tag reads is shown', async () => {
  const requests = new Map();
  const api = tagEditorSetup((command, args) => {
    const request = deferred(); requests.set(args.filePath, request); return request.promise;
  });

  const first = api.loadTagsFromPath('a.mp3');
  const second = api.loadTagsFromPath('b.mp3');
  requests.get('b.mp3').resolve({ file_path: 'b.mp3', format: 'mp3', title: 'B' });
  await second;
  requests.get('a.mp3').resolve({ file_path: 'a.mp3', format: 'mp3', title: 'A' });
  await first;
  assert.equal(api.inspect().title, 'B');
  assert.equal(api.inspect().loading, false);
});

test('a failing file or cover picker shows its error instead of rejecting', async () => {
  const api = tagEditorSetup(async command => { throw new Error(`${command} failed`); });

  await api.pickFile();
  assert.match(api.inspect().error, /pick_audio_file failed/);
  await api.pickCover();
  assert.match(api.inspect().error, /pick_cover_image failed/);
});

test('cover previews only use an image/jpeg or image/png data URL', async () => {
  const covers = [
    { cover_mime: 'image/png', cover_data: 'iVBORw0KGgo=' },
    { cover_mime: 'text/html', cover_data: 'PGI+aGk8L2I+' },
    { cover_mime: 'image/png;x=1', cover_data: 'AAAA' },
    { cover_mime: 'image/jpeg', cover_data: 'not base64!' }
  ];
  let next = 0;
  const api = tagEditorSetup(async () => ({ file_path: 'a.mp3', format: 'mp3', ...covers[next++] }));

  await api.loadTagsFromPath('a.mp3');
  assert.equal(api.inspect().coverPreviewUrl, 'data:image/png;base64,iVBORw0KGgo=');
  await api.loadTagsFromPath('a.mp3');
  assert.equal(api.inspect().coverPreviewUrl, 'data:image/jpeg;base64,PGI+aGk8L2I+');
  await api.loadTagsFromPath('a.mp3');
  assert.equal(api.inspect().coverPreviewUrl, 'data:image/jpeg;base64,AAAA');
  await api.loadTagsFromPath('a.mp3');
  assert.equal(api.inspect().coverPreviewUrl, null);
});
