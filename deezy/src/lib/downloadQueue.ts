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
  type QueuedDownload
} from './stores';
import { downloadRateLimiter } from './rateLimiter';
import { notificationManager } from './notifications';

interface DownloadResult {
  file_path: string;
  requested_quality: string;
  actual_quality: string;
}

class DownloadQueueManager {
  private processing = false;
  private activeCount = 0;
  private activeDownloadControllers = new Map<string, AbortController>();

  async addToQueue(track: Track, priority: number = 0) {
    const trackId = String(track.id);
    const currentDownloads = get(downloads);
    
    // Check if already downloading or complete (but allow paused to be re-queued)
    const state = currentDownloads.get(trackId);
    if (state === 'downloading' || state === 'complete') {
      console.log('Track already downloading or complete:', trackId);
      return;
    }

    // Remove from paused set if it was paused
    const paused = get(pausedDownloads);
    if (paused.has(trackId)) {
      paused.delete(trackId);
      pausedDownloads.set(paused);
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
    // Check if already processing to prevent multiple concurrent queue processors
    if (this.processing) {
      return;
    }
    
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
    
    // Check if paused before starting
    if (this.isPaused(trackId)) {
      console.log('Track is paused, skipping:', trackId);
      return;
    }

    this.activeCount++;
    activeDownloads.set(this.activeCount);

    // Create abort controller for this download
    const controller = new AbortController();
    this.activeDownloadControllers.set(trackId, controller);

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
          status: 'downloading',
          track: track,
          isPaused: false,
          timestamp: new Date().toISOString()
        }, ...history];
      } else {
        // Update existing entry
        return history.map(item =>
          item.trackId === trackId
            ? { ...item, status: 'downloading', percent: 0, isPaused: false, errorMsg: undefined, timestamp: new Date().toISOString() }
            : item
        );
      }
    });

    downloads.update(d => {
      d.set(trackId, 'downloading');
      return d;
    });

    try {
      // Apply rate limiting before download
      await downloadRateLimiter.throttle();

      // Check if paused again after rate limiting
      if (this.isPaused(trackId)) {
        console.log('Track was paused during rate limiting, aborting:', trackId);
        return;
      }

      const result = await invoke<DownloadResult>('download_track', { trackId });
      
      // Check if paused after download completes
      if (this.isPaused(trackId)) {
        console.log('Track was paused, not marking as complete:', trackId);
        return;
      }

      console.log('Download completed:', result.file_path);
      
      downloads.update(d => {
        d.set(trackId, 'complete');
        return d;
      });

      downloadHistory.update(history =>
        history.map(item =>
          item.trackId === trackId
            ? {
                ...item,
                percent: 100,
                status: 'complete',
                isPaused: false,
                filePath: result.file_path,
                requestedQuality: result.requested_quality,
                actualQuality: result.actual_quality
              }
            : item
        )
      );

      await notificationManager.notifyDownloadComplete(track.title, track.artist);
    } catch (err) {
      // Check if it was paused (which causes an error)
      if (this.isPaused(trackId)) {
        console.log('Download was paused:', trackId);
        return;
      }

      console.error('Download failed:', err);
      
      downloads.update(d => {
        d.set(trackId, 'error');
        return d;
      });

      downloadHistory.update(history =>
        history.map(item =>
          item.trackId === trackId
            ? { ...item, status: 'error', errorMsg: String(err), isPaused: false }
            : item
        )
      );

      await notificationManager.notifyDownloadError(track.title, track.artist, String(err));
    } finally {
      this.activeDownloadControllers.delete(trackId);
      this.activeCount--;
      activeDownloads.set(this.activeCount);
      
      // Continue processing queue if there are items remaining
      // processQueue will check if it's already processing
      if (get(downloadQueue).length > 0) {
        this.processQueue();
      }
    }
  }

  pauseDownload(trackId: string) {
    const paused = get(pausedDownloads);
    paused.add(trackId);
    pausedDownloads.set(paused);

    // Cancel the active download if it's currently downloading
    const controller = this.activeDownloadControllers.get(trackId);
    if (controller) {
      controller.abort();
      this.activeDownloadControllers.delete(trackId);
    }

    // Update download history to show paused state
    downloadHistory.update(history =>
      history.map(item =>
        item.trackId === trackId
          ? { ...item, status: 'paused', isPaused: true }
          : item
      )
    );

    // Update downloads map
    downloads.update(d => {
      d.set(trackId, 'paused');
      return d;
    });
  }

  resumeDownload(trackId: string) {
    const paused = get(pausedDownloads);
    paused.delete(trackId);
    pausedDownloads.set(paused);

    // Find the track in download history
    const history = get(downloadHistory);
    const item = history.find(h => h.trackId === trackId);
    
    if (item && item.track) {
      // Reset the download state
      downloadHistory.update(history =>
        history.map(h =>
          h.trackId === trackId
            ? { ...h, status: 'downloading', percent: 0, isPaused: false, errorMsg: undefined }
            : h
        )
      );

      // Remove from downloads map so it can be re-added
      downloads.update(d => {
        d.delete(trackId);
        return d;
      });

      // Add back to queue with high priority
      this.addToQueue(item.track, 100);
    }
  }

  isPaused(trackId: string): boolean {
    return get(pausedDownloads).has(trackId);
  }

  clearQueue() {
    downloadQueue.set([]);
  }

  getQueueLength(): number {
    return get(downloadQueue).length;
  }

  reorderQueue(newQueue: QueuedDownload[]) {
    downloadQueue.set(newQueue);
  }

  removeFromQueue(trackId: string) {
    downloadQueue.update(queue => 
      queue.filter(item => String(item.track.id) !== trackId)
    );
  }
}

export const downloadQueueManager = new DownloadQueueManager();

