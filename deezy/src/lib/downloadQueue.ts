import { invoke } from '@tauri-apps/api/core';
import { get } from 'svelte/store';
import {
  downloads,
  downloadHistory,
  downloadQueue,
  activeDownloads,
  pausedDownloads,
  MAX_CONCURRENT_DOWNLOADS,
  type Track,
  type QueuedDownload,
  type DownloadItem,
  type QualityOption
} from './stores';
import { downloadRateLimiter } from './rateLimiter';
import { notificationManager } from './notifications';

interface DownloadResult {
  file_path: string;
  requested_quality: QualityOption;
  actual_quality: QualityOption;
}

type DownloadStatus = 'downloading' | 'complete' | 'error' | 'paused' | 'resolving' | 'tagging';

const DEFAULT_PRIORITY = 0;
const HIGH_PRIORITY = 100;
const QUEUE_CHECK_INTERVAL = 1000;

class DownloadQueueManager {
  private processing = false;
  private activeCount = 0;
  private activeDownloadControllers = new Map<string, AbortController>();
  private activeTrackIds = new Set<string>();

  async addToQueue(track: Track, priority: number = DEFAULT_PRIORITY): Promise<void> {
    const trackId = String(track.id);
    const currentDownloads = get(downloads);
    
    const state = currentDownloads.get(trackId);
    if (state === 'downloading' || state === 'complete') {
      console.log('Track already downloading or complete:', trackId);
      return;
    }

    this.removeFromPausedSet(trackId);

    downloadQueue.update(queue => {
      if (this.isTrackInQueue(queue, trackId)) {
        return queue;
      }
      
      return this.sortQueueByPriority([...queue, { track, priority }]);
    });

    if (!this.processing) {
      this.processQueue();
    }
  }

  private removeFromPausedSet(trackId: string): void {
    const paused = get(pausedDownloads);
    if (paused.has(trackId)) {
      paused.delete(trackId);
      pausedDownloads.set(paused);
    }
  }

  private isTrackInQueue(queue: QueuedDownload[], trackId: string): boolean {
    return queue.some(item => String(item.track.id) === trackId);
  }

  private sortQueueByPriority(queue: QueuedDownload[]): QueuedDownload[] {
    return queue.sort((a, b) => b.priority - a.priority);
  }

  private async processQueue(): Promise<void> {
    if (this.processing) {
      return;
    }
    
    this.processing = true;

    try {
      while (true) {
        const queue = get(downloadQueue);
        
        if (queue.length === 0) {
          break;
        }

        if (this.activeCount >= MAX_CONCURRENT_DOWNLOADS) {
          await this.waitForSlot();
          continue;
        }

        const item = queue[0];
        if (!item) {
          break;
        }

        downloadQueue.update(q => q.slice(1));
        this.downloadTrack(item.track);
      }
    } finally {
      this.processing = false;
    }
  }

  private async waitForSlot(): Promise<void> {
    await new Promise(resolve => setTimeout(resolve, QUEUE_CHECK_INTERVAL));
  }

  private createDownloadHistoryItem(track: Track, trackId: string): DownloadItem {
    return {
      trackId,
      title: track.title,
      artist: track.artist,
      album: track.album,
      cover: track.cover_medium || track.cover_small,
      percent: 0,
      status: 'downloading',
      track: track,
      isPaused: false,
      timestamp: new Date().toISOString()
    };
  }

  private updateDownloadStatus(trackId: string, status: DownloadStatus): void {
    downloads.update(d => {
      d.set(trackId, status);
      return d;
    });
  }

  private updateHistoryItem(trackId: string, updates: Partial<DownloadItem>): void {
    downloadHistory.update(history =>
      history.map(item =>
        item.trackId === trackId ? { ...item, ...updates } : item
      )
    );
  }

  private addToHistory(track: Track, trackId: string): void {
    downloadHistory.update(history => {
      const existing = history.find(item => item.trackId === trackId);
      
      if (!existing) {
        return [this.createDownloadHistoryItem(track, trackId), ...history];
      }
      
      return history.map(item =>
        item.trackId === trackId
          ? { 
              ...item, 
              status: 'downloading', 
              percent: 0, 
              isPaused: false, 
              errorMsg: undefined, 
              timestamp: new Date().toISOString() 
            }
          : item
      );
    });
  }

  private async downloadTrack(track: Track): Promise<void> {
    const trackId = String(track.id);
    
    if (this.isPaused(trackId)) {
      console.log('Track is paused, skipping:', trackId);
      return;
    }

    // Increment before try so the finally block only decrements when we incremented.
    this.incrementActiveCount(trackId);
    let didIncrement = true;
    this.addToHistory(track, trackId);
    this.updateDownloadStatus(trackId, 'downloading');

    try {
      await downloadRateLimiter.throttle();

      if (this.isPaused(trackId)) {
        console.log('Track was paused during rate limiting, aborting:', trackId);
        didIncrement = false;
        this.decrementActiveCount(trackId);
        return;
      }

      const result = await invoke<DownloadResult>('download_track', { trackId });
      
      if (this.isPaused(trackId)) {
        console.log('Track was paused, not marking as complete:', trackId);
        didIncrement = false;
        this.decrementActiveCount(trackId);
        return;
      }

      console.log('Download completed:', result.file_path);
      
      this.updateDownloadStatus(trackId, 'complete');
      this.updateHistoryItem(trackId, {
        percent: 100,
        status: 'complete',
        isPaused: false,
        filePath: result.file_path,
        requestedQuality: result.requested_quality,
        actualQuality: result.actual_quality
      });

      await notificationManager.notifyDownloadComplete(track.title, track.artist);
    } catch (err) {
      if (this.isPaused(trackId)) {
        console.log('Download was paused:', trackId);
        didIncrement = false;
        this.decrementActiveCount(trackId);
        return;
      }

      console.error('Download failed:', err);
      
      this.updateDownloadStatus(trackId, 'error');
      this.updateHistoryItem(trackId, {
        status: 'error',
        errorMsg: String(err),
        isPaused: false
      });

      await notificationManager.notifyDownloadError(track.title, track.artist, String(err));
    } finally {
      if (didIncrement) {
        this.decrementActiveCount(trackId);
      }

      if (!this.processing && get(downloadQueue).length > 0) {
        this.processQueue();
      }
    }
  }

  private incrementActiveCount(trackId: string): void {
    this.activeCount++;
    this.activeTrackIds.add(trackId);
    activeDownloads.set(this.activeCount);
    
    const controller = new AbortController();
    this.activeDownloadControllers.set(trackId, controller);
  }

  private decrementActiveCount(trackId: string): void {
    this.activeDownloadControllers.delete(trackId);
    this.activeTrackIds.delete(trackId);
    this.activeCount--;
    activeDownloads.set(this.activeCount);
  }

  pauseDownload(trackId: string): void {
    const paused = get(pausedDownloads);
    paused.add(trackId);
    pausedDownloads.set(paused);

    const controller = this.activeDownloadControllers.get(trackId);
    if (controller) {
      controller.abort();
      this.activeDownloadControllers.delete(trackId);
    }

    this.updateHistoryItem(trackId, { status: 'paused', isPaused: true });
    this.updateDownloadStatus(trackId, 'paused');
  }

  resumeDownload(trackId: string): void {
    const paused = get(pausedDownloads);
    paused.delete(trackId);
    pausedDownloads.set(paused);

    const history = get(downloadHistory);
    const item = history.find(h => h.trackId === trackId);
    
    if (!item?.track) return;

    this.updateHistoryItem(trackId, {
      status: 'downloading',
      percent: 0,
      isPaused: false,
      errorMsg: undefined
    });

    downloads.update(d => {
      d.delete(trackId);
      return d;
    });

    this.addToQueue(item.track, HIGH_PRIORITY);
  }

  isPaused(trackId: string): boolean {
    return get(pausedDownloads).has(trackId);
  }

  clearQueue(): void {
    downloadQueue.set([]);
  }

  getQueueLength(): number {
    return get(downloadQueue).length;
  }

  reorderQueue(newQueue: QueuedDownload[]): void {
    downloadQueue.set(newQueue);
  }

  removeFromQueue(trackId: string): void {
    downloadQueue.update(queue => 
      queue.filter(item => String(item.track.id) !== trackId)
    );
  }

  getActiveTrackIds(): string[] {
    return Array.from(this.activeTrackIds);
  }

  getActiveCount(): number {
    return this.activeCount;
  }

  isProcessing(): boolean {
    return this.processing;
  }
}

export const downloadQueueManager = new DownloadQueueManager();

