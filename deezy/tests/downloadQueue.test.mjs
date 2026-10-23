import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, loadModule, settle, store } from './helpers.mjs';

function setup(completionNotice = Promise.resolve()) {
  const stores = {
    downloads: store(new Map()), downloadHistory: store([]), downloadQueue: store([]),
    activeDownloads: store(0), pausedDownloads: store(new Set()), downloadDelaySeconds: store(0),
    maxConcurrentDownloads: store(3)
  };
  const download = deferred();
  const cancel = deferred();
  // Each download_track call gets its own deferred, keyed by track ID.
  const downloadsById = new Map();
  let notifications = 0;
  const { downloadQueueManager: manager } = loadModule('../src/lib/downloadQueue.ts', {
    './stores': stores,
    'svelte/store': { get: s => s.value },
    '@tauri-apps/api/core': { invoke: (command, args) => {
      if (command !== 'download_track') return cancel.promise;
      if (args.trackId === '1') return download.promise;
      const pending = deferred();
      downloadsById.set(args.trackId, pending);
      return pending.promise;
    } },
    './rateLimiter': { downloadRateLimiter: { throttle: async () => {} } },
    './notifications': { notificationManager: {
      notifyDownloadComplete: () => { notifications++; return completionNotice; },
      notifyDownloadError: async () => {}
    } }
  });
  return { manager, stores, download, cancel, downloadsById, notifications: () => notifications };
}

const track = { id: 1, title: 'Track', artist: 'Artist', album: 'Album' };
const complete = { status: 'complete', file_path: 'track.mp3', requested_quality: 'MP3_320', actual_quality: 'MP3_320' };

test('a late cancellation response cannot change a completed download to paused', async () => {
  const { manager, stores, download, cancel } = setup();
  await manager.addToQueue(track);
  const pausing = manager.pauseDownload('1');
  download.resolve(complete);
  await settle();
  cancel.resolve(false);
  await pausing;
  assert.equal(stores.downloads.value.get('1'), 'complete');
  assert.equal(stores.downloadHistory.value[0].isPaused, false);
  assert.equal(stores.activeDownloads.value, 0);
});

test('clearing history preserves active downloads and their eventual file paths', async () => {
  const { manager, stores, download } = setup();
  stores.downloadHistory.set([{ trackId: '2', status: 'complete' }]);
  stores.downloads.value.set('2', 'complete');
  await manager.addToQueue(track);
  manager.clearHistory();
  assert.equal(stores.downloadHistory.value.length, 1);
  assert.equal(stores.downloadHistory.value[0].trackId, '1');
  assert.equal(stores.downloads.value.has('2'), false);
  download.resolve(complete);
  await settle();
  assert.equal(stores.downloadHistory.value[0].filePath, 'track.mp3');
});

test('failed cancellation does not leave an active download marked paused', async () => {
  const { manager, stores, download, cancel } = setup();
  await manager.addToQueue(track);
  const pausing = manager.pauseDownload('1');
  cancel.reject(new Error('IPC failed'));
  await pausing;
  assert.equal(manager.isPaused('1'), false);
  assert.equal(stores.downloadHistory.value[0].isPaused, false);
  download.resolve(complete);
  await settle();
});

test('pause requests for inactive completed tracks are ignored', async () => {
  const { manager, stores } = setup();
  stores.downloads.value.set('1', 'complete');
  await manager.pauseDownload('1');
  assert.equal(stores.downloads.value.get('1'), 'complete');
  assert.equal(manager.isPaused('1'), false);
});

test('a completed track waiting for notification permission cannot be paused', async () => {
  const notice = deferred();
  const { manager, stores, download } = setup(notice.promise);
  await manager.addToQueue(track);
  download.resolve(complete);
  await settle();
  assert.equal(stores.activeDownloads.value, 0);
  await manager.pauseDownload('1');
  assert.equal(stores.downloads.value.get('1'), 'complete');
  notice.resolve();
  await settle();
  assert.equal(stores.activeDownloads.value, 0);
});

test('all interrupted phases recover as resumable without mutating live state', () => {
  const { recoverDownloadHistory } = loadModule('../src/lib/downloadHistory.ts', {});
  const original = ['resolving', 'downloading', 'tagging', 'complete', 'error'].map((status, i) => ({ trackId: String(i), status, track }));
  const recovered = recoverDownloadHistory(original);
  assert.equal(recovered.slice(0, 3).every(item => item.status === 'paused' && item.isPaused && item.track === track), true);
  assert.equal(original[0].status, 'resolving');
  assert.equal(recovered[3].status, 'complete');
  assert.equal(recovered[4].status, 'error');
});

test('a skipped track is complete without a quality or notification and does not hold up the next start', async () => {
  const { manager, stores, download, downloadsById, notifications } = setup();
  stores.downloadDelaySeconds.set(60);
  await manager.addToQueue(track);
  await manager.addToQueue({ ...track, id: 2 });
  await settle();
  assert.equal(downloadsById.has('2'), false, 'the delay holds back the second start');

  download.resolve({ status: 'skipped', file_path: 'existing.mp3', requested_quality: 'MP3_320', actual_quality: '' });
  await settle();
  const row = stores.downloadHistory.value.find(item => item.trackId === '1');
  assert.equal(row.status, 'complete');
  assert.equal(row.skipped, true);
  assert.equal(row.filePath, 'existing.mp3');
  assert.equal(row.actualQuality, undefined);
  assert.equal(notifications(), 0);

  // The queue rechecks the delay at least once per second.
  await new Promise(resolve => setTimeout(resolve, 1100));
  assert.equal(downloadsById.has('2'), true, 'the skip released the delay');
  downloadsById.get('2').resolve(complete);
  await settle();
});

test('the simultaneous download limit holds back extra tracks and applies when it is raised', async () => {
  const { manager, stores, download, downloadsById } = setup();
  stores.maxConcurrentDownloads.set(1);
  await manager.addToQueue(track);
  await manager.addToQueue({ ...track, id: 2 });
  await settle();
  assert.equal(stores.activeDownloads.value, 1);
  assert.equal(downloadsById.has('2'), false, 'the limit holds back the second track');

  stores.maxConcurrentDownloads.set(2);
  // The queue rechecks for a free slot once per second.
  await new Promise(resolve => setTimeout(resolve, 1100));
  assert.equal(downloadsById.has('2'), true, 'the raised limit started the queued track');
  assert.equal(stores.activeDownloads.value, 2);
  download.resolve(complete);
  downloadsById.get('2').resolve(complete);
  await settle();
  assert.equal(stores.activeDownloads.value, 0);
});
