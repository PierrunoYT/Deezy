import { writable, type Writable } from 'svelte/store';

export type DownloadStatus = 'downloading' | 'complete' | 'error' | 'paused' | 'resolving' | 'tagging';
export type Theme = 'light' | 'dark' | 'system' | 'custom';
export type QualityOption = 'MP3_128' | 'MP3_320' | 'FLAC';

export interface UserInfo {
  id: number;
  name: string;
  image: string | null;
  is_free_account?: boolean;
}

export interface Track {
  id: number;
  title: string;
  artist: string;
  artist_id: number;
  album: string;
  duration: number;
  cover_small: string;
  cover_medium: string;
  preview?: string;
}

export interface DownloadItem {
  trackId: string;
  title: string;
  artist: string;
  album: string;
  cover: string;
  percent: number;
  status: DownloadStatus;
  errorMsg?: string;
  track?: Track;
  isPaused?: boolean;
  timestamp?: string;
  filePath?: string;
  requestedQuality?: QualityOption;
  actualQuality?: QualityOption;
}

export interface QueuedDownload {
  track: Track;
  priority: number;
}

export interface AudioPlayerState {
  currentTrack: Track | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
}

export const MAX_CONCURRENT_DOWNLOADS = 3;
export const DEFAULT_VOLUME = 0.7;
export const DEFAULT_THEME: Theme = 'dark';
export const DEFAULT_LOCALE = 'en';

export const loggedIn: Writable<boolean> = writable(false);
export const userInfo: Writable<UserInfo | null> = writable(null);
export const downloads: Writable<Map<string, string>> = writable(new Map());
export const activeDownloads: Writable<number> = writable(0);
export const downloadHistory: Writable<DownloadItem[]> = writable([]);
export const downloadQueue: Writable<QueuedDownload[]> = writable([]);
export const pausedDownloads: Writable<Set<string>> = writable(new Set());

export const theme: Writable<Theme> = writable(DEFAULT_THEME);
export const notificationsEnabled: Writable<boolean> = writable(true);
export const searchHistory: Writable<string[]> = writable([]);
export const currentLocale: Writable<string> = writable(DEFAULT_LOCALE);
export const settingsArlDraft: Writable<string> = writable('');

export const audioPlayer: Writable<AudioPlayerState> = writable({
  currentTrack: null,
  isPlaying: false,
  currentTime: 0,
  duration: 0,
  volume: DEFAULT_VOLUME,
});

