import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import { activeDownloads, downloadQueue, pausedDownloads } from './stores';
import { downloadQueueManager } from './downloadQueue';

const DEFAULT_TOOLTIP = 'Deezy';

class TrayManager {
  private initialized = false;
  private unlistenPauseResume: UnlistenFn | undefined;
  private unsubscribeActiveDownloads: (() => void) | undefined;
  private unsubscribeDownloadQueue: (() => void) | undefined;
  private unsubscribePausedDownloads: (() => void) | undefined;
  private updateDebounceTimer: ReturnType<typeof setTimeout> | undefined;
  private readonly debounceDelay = 100;

  async init(): Promise<void> {
    if (this.initialized) return;

    this.unlistenPauseResume = await listen('tray-pause-resume', () => {
      this.togglePauseResume();
    });

    this.unsubscribeActiveDownloads = activeDownloads.subscribe(() => {
      this.debouncedUpdateTrayStatus();
    });

    this.unsubscribeDownloadQueue = downloadQueue.subscribe(() => {
      this.debouncedUpdateTrayStatus();
    });

    this.unsubscribePausedDownloads = pausedDownloads.subscribe(() => {
      this.debouncedUpdateTrayStatus();
    });

    this.initialized = true;
  }

  destroy(): void {
    this.unlistenPauseResume?.();
    this.unlistenPauseResume = undefined;
    this.unsubscribeActiveDownloads?.();
    this.unsubscribeActiveDownloads = undefined;
    this.unsubscribeDownloadQueue?.();
    this.unsubscribeDownloadQueue = undefined;
    this.unsubscribePausedDownloads?.();
    this.unsubscribePausedDownloads = undefined;
    if (this.updateDebounceTimer) {
      clearTimeout(this.updateDebounceTimer);
      this.updateDebounceTimer = undefined;
    }
    this.initialized = false;
  }

  private debouncedUpdateTrayStatus(): void {
    if (this.updateDebounceTimer) {
      clearTimeout(this.updateDebounceTimer);
    }
    
    this.updateDebounceTimer = setTimeout(() => {
      this.updateTrayStatus();
    }, this.debounceDelay);
  }

  private togglePauseResume(): void {
    const paused = get(pausedDownloads);
    const active = get(activeDownloads);
    const queue = get(downloadQueue);

    if (paused.size > 0) {
      this.resumeAllDownloads(paused);
    } else if (active > 0 || queue.length > 0) {
      this.pauseAllDownloads(queue);
    }
  }

  private resumeAllDownloads(paused: Set<string>): void {
    paused.forEach(trackId => {
      downloadQueueManager.resumeDownload(trackId);
    });
  }

  private pauseAllDownloads(queue: any[]): void {
    const activeTrackIds = downloadQueueManager.getActiveTrackIds();
    
    activeTrackIds.forEach(trackId => {
      void downloadQueueManager.pauseDownload(trackId);
    });
    
    queue.forEach(item => {
      void downloadQueueManager.pauseDownload(String(item.track.id));
    });
  }

  private async updateTrayStatus(): Promise<void> {
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

      const tooltip = this.buildTooltip(active, queue.length, paused.size, downloadsActive, downloadsPaused);
      await invoke('set_tray_tooltip', { tooltip });
    } catch (error) {
      console.error('Failed to update tray status:', error);
    }
  }

  private buildTooltip(
    active: number, 
    queueLength: number, 
    pausedCount: number,
    downloadsActive: boolean,
    downloadsPaused: boolean
  ): string {
    if (downloadsActive) {
      const totalDownloads = active + queueLength;
      const plural = totalDownloads !== 1 ? 's' : '';
      return `${DEFAULT_TOOLTIP} - ${totalDownloads} download${plural} in progress`;
    }
    
    if (downloadsPaused) {
      const plural = pausedCount !== 1 ? 's' : '';
      return `${DEFAULT_TOOLTIP} - ${pausedCount} download${plural} paused`;
    }
    
    return DEFAULT_TOOLTIP;
  }

  async updateTooltip(text: string): Promise<void> {
    try {
      await invoke('set_tray_tooltip', { tooltip: text });
    } catch (error) {
      console.error('Failed to update tray tooltip:', error);
    }
  }
}

export const trayManager = new TrayManager();
