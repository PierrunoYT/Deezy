import { writable } from 'svelte/store';

export interface DownloadItem {
  trackId: string;
  title: string;
  artist: string;
  album: string;
  cover: string;
  percent: number;
  status: string;
  errorMsg?: string;
}

export const loggedIn = writable(false);
export const userInfo = writable<any>(null);
export const downloads = writable<Map<string, string>>(new Map());
export const activeDownloads = writable(0);
export const downloadHistory = writable<DownloadItem[]>([]);
