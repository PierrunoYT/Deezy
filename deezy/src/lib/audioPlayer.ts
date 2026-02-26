/**
 * Audio player manager for preview playback
 */

import { audioPlayer, type Track, type AudioPlayerState } from './stores';
import { get } from 'svelte/store';

const DEFAULT_VOLUME = 0.7;
const MIN_VOLUME = 0;
const MAX_VOLUME = 1;

type AudioEventHandler = () => void;
type AudioErrorHandler = (e: Event) => void;

class AudioPlayerManager {
  private audio: HTMLAudioElement | null = null;
  private endedHandler: AudioEventHandler | null = null;
  private timeupdateHandler: AudioEventHandler | null = null;
  private loadedmetadataHandler: AudioEventHandler | null = null;
  private errorHandler: AudioErrorHandler | null = null;

  constructor() {
    if (typeof window !== 'undefined') {
      this.initializeAudioElement();
    }
  }

  private initializeAudioElement(): void {
    this.audio = new Audio();
    this.audio.volume = DEFAULT_VOLUME;
    
    this.endedHandler = () => this.stop();

    this.timeupdateHandler = () => {
      if (!this.audio) return;
      
      audioPlayer.update(state => ({
        ...state,
        currentTime: this.audio!.currentTime,
        duration: this.audio!.duration || 0,
      }));
    };

    this.loadedmetadataHandler = () => {
      if (!this.audio) return;
      
      audioPlayer.update(state => ({
        ...state,
        duration: this.audio!.duration || 0,
      }));
    };

    this.errorHandler = (e: Event) => {
      console.error('Audio playback error:', e);
      this.stop();
    };

    this.attachEventListeners();
  }

  private attachEventListeners(): void {
    if (!this.audio) return;

    if (this.endedHandler) {
      this.audio.addEventListener('ended', this.endedHandler);
    }
    if (this.timeupdateHandler) {
      this.audio.addEventListener('timeupdate', this.timeupdateHandler);
    }
    if (this.loadedmetadataHandler) {
      this.audio.addEventListener('loadedmetadata', this.loadedmetadataHandler);
    }
    if (this.errorHandler) {
      this.audio.addEventListener('error', this.errorHandler);
    }
  }

  private detachEventListeners(): void {
    if (!this.audio) return;

    if (this.endedHandler) {
      this.audio.removeEventListener('ended', this.endedHandler);
    }
    if (this.timeupdateHandler) {
      this.audio.removeEventListener('timeupdate', this.timeupdateHandler);
    }
    if (this.loadedmetadataHandler) {
      this.audio.removeEventListener('loadedmetadata', this.loadedmetadataHandler);
    }
    if (this.errorHandler) {
      this.audio.removeEventListener('error', this.errorHandler);
    }
  }

  destroy(): void {
    if (this.audio) {
      this.stop();
      this.detachEventListeners();
      
      this.audio = null;
      this.endedHandler = null;
      this.timeupdateHandler = null;
      this.loadedmetadataHandler = null;
      this.errorHandler = null;
    }
  }

  private isSameTrack(track: Track): boolean {
    const currentState = get(audioPlayer);
    return currentState.currentTrack?.id === track.id;
  }

  private updatePlayerState(updates: Partial<AudioPlayerState>): void {
    audioPlayer.update(state => ({ ...state, ...updates }));
  }

  private clampVolume(volume: number): number {
    return Math.max(MIN_VOLUME, Math.min(MAX_VOLUME, volume));
  }

  private clampTime(time: number, duration: number): number {
    return Math.max(0, Math.min(time, duration));
  }

  play(track: Track): void {
    if (!this.audio) {
      console.warn('Audio player not initialized');
      return;
    }

    if (!track.preview) {
      console.warn('No preview available for track:', track.title);
      return;
    }

    if (this.isSameTrack(track)) {
      this.togglePlayPause();
      return;
    }

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

  pause(): void {
    if (!this.audio || this.audio.paused) return;

    this.audio.pause();
    this.updatePlayerState({ isPlaying: false });
  }

  resume(): void {
    if (!this.audio || !this.audio.paused) return;

    this.audio.play().catch(err => {
      console.error('Failed to resume audio:', err);
      this.stop();
    });
    
    this.updatePlayerState({ isPlaying: true });
  }

  stop(): void {
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
      volume: this.audio?.volume ?? DEFAULT_VOLUME,
    });
  }

  togglePlayPause(): void {
    const state = get(audioPlayer);
    
    if (state.isPlaying) {
      this.pause();
    } else if (state.currentTrack) {
      this.resume();
    }
  }

  seek(time: number): void {
    if (!this.audio) return;

    const duration = this.audio.duration || 0;
    const clampedTime = this.clampTime(time, duration);
    
    this.audio.currentTime = clampedTime;
    this.updatePlayerState({ currentTime: clampedTime });
  }

  setVolume(volume: number): void {
    if (!this.audio) return;

    const clampedVolume = this.clampVolume(volume);
    this.audio.volume = clampedVolume;
    this.updatePlayerState({ volume: clampedVolume });
  }

  getCurrentTrack(): Track | null {
    return get(audioPlayer).currentTrack;
  }

  isPlaying(): boolean {
    return get(audioPlayer).isPlaying;
  }

  getVolume(): number {
    return this.audio?.volume ?? DEFAULT_VOLUME;
  }

  getDuration(): number {
    return this.audio?.duration ?? 0;
  }

  getCurrentTime(): number {
    return this.audio?.currentTime ?? 0;
  }
}

export const audioPlayerManager = new AudioPlayerManager();
