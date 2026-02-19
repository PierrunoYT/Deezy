import { writable } from 'svelte/store';

export interface UserInfo {
  id: number;
  name: string;
  image: string;
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
}

export interface Track {
  id: number;
  title: string;
  artist: string;
  album: string;
  duration: number;
  cover_small: string;
  cover_medium: string;
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
export const MAX_CONCURRENT_DOWNLOADS = 3;
