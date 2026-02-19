/**
 * Audio player manager for preview playback
 */

import { audioPlayer, type Track } from './stores';
import { get } from 'svelte/store';

class AudioPlayerManager {
  private audio: HTMLAudioElement | null = null;
  private updateInterval: ReturnType<typeof setInterval> | null = null;

  constructor() {
    if (typeof window !== 'undefined') {
      this.audio = new Audio();
      this.audio.volume = 0.7;
      
      this.audio.addEventListener('ended', () => {
        this.stop();
      });

      this.audio.addEventListener('timeupdate', () => {
        if (this.audio) {
          audioPlayer.update(state => ({
            ...state,
            currentTime: this.audio!.currentTime,
            duration: this.audio!.duration || 0,
          }));
        }
      });

      this.audio.addEventListener('loadedmetadata', () => {
        if (this.audio) {
          audioPlayer.update(state => ({
            ...state,
            duration: this.audio!.duration || 0,
          }));
        }
      });

      this.audio.addEventListener('error', (e) => {
        console.error('Audio playback error:', e);
        this.stop();
      });
    }
  }

  play(track: Track) {
    if (!this.audio || !track.preview) {
      console.warn('No preview available for this track');
      return;
    }

    const currentState = get(audioPlayer);
    
    // If same track, just toggle play/pause
    if (currentState.currentTrack?.id === track.id) {
      if (currentState.isPlaying) {
        this.pause();
      } else {
        this.resume();
      }
      return;
    }

    // Stop current track and play new one
    this.stop();
    
    this.audio.src = track.preview;
    this.audio.load();
    
    audioPlayer.set({
      currentTrack: track,
      isPlaying: true,
      currentTime: 0,
      duration: 0,
      volume: this.audio.volume,
    });

    this.audio.play().catch(err => {
      console.error('Failed to play audio:', err);
      this.stop();
    });
  }

  pause() {
    if (this.audio && !this.audio.paused) {
      this.audio.pause();
      audioPlayer.update(state => ({
        ...state,
        isPlaying: false,
      }));
    }
  }

  resume() {
    if (this.audio && this.audio.paused) {
      this.audio.play().catch(err => {
        console.error('Failed to resume audio:', err);
        this.stop();
      });
      audioPlayer.update(state => ({
        ...state,
        isPlaying: true,
      }));
    }
  }

  stop() {
    if (this.audio) {
      this.audio.pause();
      this.audio.currentTime = 0;
      this.audio.src = '';
    }
    
    audioPlayer.set({
      currentTrack: null,
      isPlaying: false,
      currentTime: 0,
      duration: 0,
      volume: this.audio?.volume || 0.7,
    });
  }

  togglePlayPause() {
    const state = get(audioPlayer);
    if (state.isPlaying) {
      this.pause();
    } else if (state.currentTrack) {
      this.resume();
    }
  }

  seek(time: number) {
    if (this.audio) {
      this.audio.currentTime = time;
      audioPlayer.update(state => ({
        ...state,
        currentTime: time,
      }));
    }
  }

  setVolume(volume: number) {
    if (this.audio) {
      this.audio.volume = Math.max(0, Math.min(1, volume));
      audioPlayer.update(state => ({
        ...state,
        volume: this.audio!.volume,
      }));
    }
  }

  getCurrentTrack(): Track | null {
    return get(audioPlayer).currentTrack;
  }

  isPlaying(): boolean {
    return get(audioPlayer).isPlaying;
  }
}

export const audioPlayerManager = new AudioPlayerManager();
