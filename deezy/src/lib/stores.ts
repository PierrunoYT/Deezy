import { writable } from 'svelte/store';

export interface UserInfo {
  id: number;
  name: string;
  image: string;
  is_free_account?: boolean;
}

export interface DownloadItem {
  trackId: string;
  title: string;
  artist: string;
  album: string;
  cover: string;
  percent: number;
  status: string;
  errorMsg?: string;
  track?: Track;
  isPaused?: boolean;
  timestamp?: string;
  filePath?: string;
  requestedQuality?: string;
  actualQuality?: string;
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

export interface QueuedDownload {
  track: Track;
  priority: number;
}

export const loggedIn = writable<boolean>(false);
export const userInfo = writable<UserInfo | null>(null);
export const downloads = writable<Map<string, string>>(new Map());
export const activeDownloads = writable<number>(0);
export const downloadHistory = writable<DownloadItem[]>([]);
export const downloadQueue = writable<QueuedDownload[]>([]);
export const pausedDownloads = writable<Set<string>>(new Set());
export const MAX_CONCURRENT_DOWNLOADS = 3;

export type Theme = 'light' | 'dark' | 'system' | 'custom';
export const theme = writable<Theme>('dark');
export const notificationsEnabled = writable<boolean>(true);

export const searchHistory = writable<string[]>([]);
export const currentLocale = writable<string>('en');

// Audio player state
export interface AudioPlayerState {
  currentTrack: Track | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
}

export const audioPlayer = writable<AudioPlayerState>({
  currentTrack: null,
  isPlaying: false,
  currentTime: 0,
  duration: 0,
  volume: 0.7,
});

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
}

export interface UpdateState {
  available: boolean;
  checking: boolean;
  downloading: boolean;
  downloadProgress: number;
  error: string | null;
  updateInfo: UpdateInfo | null;
}

export const updateState = writable<UpdateState>({
  available: false,
  checking: false,
  downloading: false,
  downloadProgress: 0,
  error: null,
  updateInfo: null,
});
