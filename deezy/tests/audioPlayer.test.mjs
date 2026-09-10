import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, loadModule, settle, store } from './helpers.mjs';

function setup() {
  const audioPlayer = store({ currentTrack: null });
  const attempts = [];
  let audio;
  class Audio {
    constructor() { audio = this; this.paused = true; this.volume = 0.7; this.duration = NaN; }
    addEventListener() {}
    removeEventListener() {}
    removeAttribute(name) { delete this[name]; }
    load() {}
    pause() { this.paused = true; }
    play() { this.paused = false; const pending = deferred(); attempts.push(pending); return pending.promise; }
  }
  const { audioPlayerManager: manager } = loadModule('../src/lib/audioPlayer.ts', {
    './stores': { audioPlayer }, 'svelte/store': { get: s => s.value }
  }, { window: {}, Audio });
  return { manager, audioPlayer, attempts, audio };
}

const track = id => ({ id, title: `Track ${id}`, preview: `https://example.test/${id}.mp3` });

test('an aborted previous preview cannot stop the newly selected track', async () => {
  const { manager, attempts, audioPlayer } = setup();
  manager.play(track(1));
  manager.play(track(2));
  attempts[0].reject(new Error('aborted by load'));
  await settle();
  assert.equal(audioPlayer.value.currentTrack.id, 2);
  assert.equal(audioPlayer.value.isPlaying, true);
});

test('pause invalidates pending playback without clearing the selected track', async () => {
  const { manager, attempts, audioPlayer } = setup();
  manager.play(track(1));
  manager.pause();
  attempts[0].reject(new Error('aborted by pause'));
  await settle();
  assert.equal(audioPlayer.value.currentTrack.id, 1);
  assert.equal(audioPlayer.value.isPlaying, false);
});

test('a current playback failure clears the player', async () => {
  const { manager, attempts, audioPlayer, audio } = setup();
  manager.play(track(1));
  attempts[0].reject(new Error('unsupported media'));
  await settle();
  assert.equal(audioPlayer.value.currentTrack, null);
  assert.equal(audio.src, undefined);
});

test('non-finite media controls do not reach the audio element', () => {
  const { manager, audio } = setup();
  manager.play(track(1));
  manager.seek(12);
  manager.setVolume(NaN);
  assert.equal(audio.currentTime, 0);
  assert.equal(audio.volume, 0.7);
  audio.duration = 30;
  manager.seek(Infinity);
  assert.equal(audio.currentTime, 0);
  manager.seek(60);
  assert.equal(audio.currentTime, 30);
});
