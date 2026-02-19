import { invoke } from '@tauri-apps/api/core';
import { get } from 'svelte/store';
import {
  downloads,
  downloadHistory,
  downloadQueue,
  activeDownloads,
  MAX_CONCURRENT_DOWNLOADS,
  type Track,
  type QueuedDownload
} from './stores';
import { downloadRateLimiter } from './rateLimiter';

class DownloadQueueManager {
  private processing = false;
  private activeCount = 0;

  async addToQueue(track: Track, priority: number = 0) {
    const trackId = String(track.id);
    const currentDownloads = get(downloads);
    
    // Check if already downloading or complete
    const state = currentDownloads.get(trackId);
    if (state === 'downloading' || state === 'complete') {
      console.log('Track already downloading or complete:', trackId);
      return;
    }

    // Add to queue
    downloadQueue.update(queue => {
      // Check if already in queue
      if (queue.some(item => String(item.track.id) === trackId)) {
        return queue;
      }
      
      const newQueue = [...queue, { track, priority }];
      // Sort by priority (higher first)
      newQueue.sort((a, b) => b.priority - a.priority);
      return newQueue;
    });

    // Start processing if not already
    if (!this.processing) {
      this.processQueue();
    }
  }

  private async processQueue() {
    this.processing = true;

    while (true) {
      const queue = get(downloadQueue);
      
      // Stop if queue is empty
      if (queue.length === 0) {
        this.processing = false;
        break;
      }

      // Wait if we're at max concurrent downloads
      if (this.activeCount >= MAX_CONCURRENT_DOWNLOADS) {
        await new Promise(resolve => setTimeout(resolve, 1000));
        continue;
      }

      // Get next item from queue
      const item = queue[0];
      if (!item) {
        this.processing = false;
        break;
      }

      // Remove from queue
      downloadQueue.update(q => q.slice(1));

      // Start download (don't await - let it run in background)
      this.downloadTrack(item.track);
    }
  }

  private async downloadTrack(track: Track) {
    const trackId = String(track.id);
    this.activeCount++;
    activeDownloads.set(this.activeCount);

    // Add to download history with initial state
    downloadHistory.update(history => {
      const existing = history.find(item => item.trackId === trackId);
      if (!existing) {
        return [{
          trackId,
          title: track.title,
          artist: track.artist,
          album: track.album,
          cover: track.cover_medium || track.cover_small,
          percent: 0,
          status: 'downloading'
        }, ...history];
      }
      return history;
    });

    downloads.update(d => {
      d.set(trackId, 'downloading');
      return d;
    });

    try {
      // Apply rate limiting before download
      await downloadRateLimiter.throttle();

      const result = await invoke<string>('download_track', { trackId });
      console.log('Download completed:', result);
      
      downloads.update(d => {
        d.set(trackId, 'complete');
        return d;
      });

      // Always mark the history entry as 100 % complete regardless of
      // whether every Tauri progress event was received while the
      // DownloadsView was mounted.
      downloadHistory.update(history =>
        history.map(item =>
          item.trackId === trackId
            ? { ...item, percent: 100, status: 'complete' }
            : item
        )
      );
    } catch (err) {
      console.error('Download failed:', err);
      
      downloads.update(d => {
        d.set(trackId, 'error');
        return d;
      });

      // Update download history with error
      downloadHistory.update(history =>
        history.map(item =>
          item.trackId === trackId
            ? { ...item, status: 'error', errorMsg: String(err) }
            : item
        )
      );
    } finally {
      this.activeCount--;
      activeDownloads.set(this.activeCount);
      
      // Continue processing queue
      if (!this.processing && get(downloadQueue).length > 0) {
        this.processQueue();
      }
    }
  }

  clearQueue() {
    downloadQueue.set([]);
  }

  getQueueLength(): number {
    return get(downloadQueue).length;
  }
}

export const downloadQueueManager = new DownloadQueueManager();

