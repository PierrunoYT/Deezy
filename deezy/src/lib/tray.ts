import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import { activeDownloads, downloadQueue, pausedDownloads } from './stores';
import { downloadQueueManager } from './downloadQueue';

class TrayManager {
  private initialized = false;

  async init() {
    if (this.initialized) return;

    // Listen for tray pause/resume event
    await listen('tray-pause-resume', () => {
      this.togglePauseResume();
    });

    // Subscribe to download state changes
    activeDownloads.subscribe(() => {
      this.updateTrayStatus();
    });

    downloadQueue.subscribe(() => {
      this.updateTrayStatus();
    });

    pausedDownloads.subscribe(() => {
      this.updateTrayStatus();
    });

    this.initialized = true;
  }

  private togglePauseResume() {
    const paused = get(pausedDownloads);
    const active = get(activeDownloads);
    const queue = get(downloadQueue);

    // If there are paused downloads, resume them
    if (paused.size > 0) {
      paused.forEach(trackId => {
        downloadQueueManager.resumeDownload(trackId);
      });
    } else if (active > 0 || queue.length > 0) {
      // Pause all active downloads
      const activeTrackIds = downloadQueueManager.getActiveTrackIds();
      activeTrackIds.forEach(trackId => {
        downloadQueueManager.pauseDownload(trackId);
      });
      
      // Pause all queued downloads
      queue.forEach(item => {
        downloadQueueManager.pauseDownload(String(item.track.id));
      });
    }
  }

  private async updateTrayStatus() {
    const active = get(activeDownloads);
    const queue = get(downloadQueue);
    const paused = get(pausedDownloads);

    const downloadsActive = active > 0 || queue.length > 0;
    const downloadsPaused = paused.size > 0;

    try {
      await invoke('update_tray_status', {
        downloadsActive,
        downloadsPaused
      });

      // Update tooltip
      let tooltip = 'Deezy';
      if (downloadsActive) {
        const totalDownloads = active + queue.length;
        tooltip = `Deezy - ${totalDownloads} download${totalDownloads !== 1 ? 's' : ''} in progress`;
      } else if (downloadsPaused) {
        tooltip = `Deezy - ${paused.size} download${paused.size !== 1 ? 's' : ''} paused`;
      }

      await invoke('set_tray_tooltip', { tooltip });
    } catch (error) {
      console.error('Failed to update tray status:', error);
    }
  }
}

export const trayManager = new TrayManager();
