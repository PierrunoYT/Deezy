<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import { downloadHistory, downloads, type DownloadItem, type DownloadStatus } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import QueueView from './QueueView.svelte';
  import ExportHistoryModal from './ExportHistoryModal.svelte';
  import { _ } from 'svelte-i18n';

  interface DownloadProgressEvent {
    track_id: string;
    title: string;
    percent: number;
    status: DownloadStatus;
  }

  interface TagErrorEvent {
    track_id: string;
    title: string;
    error: string;
  }

  let downloadItems = $state<DownloadItem[]>([]);
  let showExportModal = $state(false);
  let unlistenProgress: UnlistenFn | undefined;
  let unlistenTagError: UnlistenFn | undefined;

  $effect(() => {
    const unsubHistory = downloadHistory.subscribe(val => {
      downloadItems = val;
    });
    return unsubHistory;
  });

  onMount(async () => {
    unlistenProgress = await listen<DownloadProgressEvent>('download-progress', (event) => {
      const { track_id, title, percent, status } = event.payload;

      downloadHistory.update(history => {
        const existingIndex = history.findIndex(d => d.trackId === track_id);

        if (existingIndex >= 0) {
          const oldItem = history[existingIndex];
          if (
            oldItem.title === title &&
            oldItem.percent === percent &&
            oldItem.status === status
          ) {
            return history;
          }
          return history.map((item, idx) =>
            idx === existingIndex
              ? { ...item, title, percent, status }
              : item
          );
        }
        return history;
      });
    });

    unlistenTagError = await listen<TagErrorEvent>('tag-writing-error', (event) => {
      const { track_id, error } = event.payload;
      downloadHistory.update(history =>
        history.map(item =>
          item.trackId === track_id
            ? { ...item, errorMsg: `Warning: Tags not written - ${error}` }
            : item
        )
      );
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenTagError?.();
  });

  function clearHistory(): void {
    downloadHistory.set([]);
    downloads.update(d => {
      d.clear();
      return d;
    });
  }

  function retryDownload(item: DownloadItem): void {
    if (!item.track) return;

    downloadHistory.update(history =>
      history.map(h =>
        h.trackId === item.trackId
          ? { ...h, status: 'downloading', percent: 0, errorMsg: undefined }
          : h
      )
    );

    downloads.update(d => {
      d.delete(item.trackId);
      return d;
    });

    downloadQueueManager.addToQueue(item.track);
  }

  function pauseDownload(item: DownloadItem): void {
    if (!item.trackId) return;
    downloadQueueManager.pauseDownload(item.trackId);
  }

  function resumeDownload(item: DownloadItem): void {
    if (!item.trackId) return;
    downloadQueueManager.resumeDownload(item.trackId);
  }

  async function openDownloadedFile(item: DownloadItem): Promise<void> {
    if (!item.filePath) return;

    try {
      await invoke('show_in_folder', { filePath: item.filePath });
    } catch (err) {
      console.error('Failed to show file in folder:', err);
    }
  }

  function getStatusText(status: string, percent: number): string {
    const statusMap: Record<string, string> = {
      complete: $_('downloads.status.complete'),
      error: $_('downloads.status.error'),
      paused: $_('downloads.status.paused'),
      tagging: $_('downloads.status.tagging'),
      resolving: $_('downloads.status.resolving')
    };

    if (status === 'downloading') {
      return $_('downloads.status.downloading', { values: { percent: Math.round(percent ?? 0) } });
    }

    return statusMap[status] ?? status;
  }

  function formatQualityLabel(value?: string): string {
    if (!value) return '';
    return value.replace('MP3_', 'MP3 ').replace('_', ' ');
  }

  function getProgressWidth(percent?: number): number {
    return Math.max(0, Math.min(percent ?? 0, 100));
  }

  function getStatusClass(status: string): string {
    return ['complete', 'error', 'paused'].includes(status) ? status : '';
  }

  function shouldShowActionButton(status: string): boolean {
    return ['downloading', 'resolving', 'paused', 'error'].includes(status);
  }
</script>

<div class="view">
  <div class="header-row">
    <h2>{$_('downloads.title')}</h2>
    {#if downloadItems.length > 0}
      <div class="header-actions">
        <button 
          class="action-btn export-btn" 
          type="button" 
          onclick={() => showExportModal = true}
          aria-label={$_('downloads.exportHistory')}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          {$_('downloads.exportHistory')}
        </button>
        <button 
          class="action-btn clear-btn" 
          type="button" 
          onclick={clearHistory}
          aria-label={$_('downloads.clearHistory')}
        >
          {$_('downloads.clearHistory')}
        </button>
      </div>
    {/if}
  </div>

  <QueueView />

  {#if downloadItems.length === 0}
    <div class="empty-state">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
        <polyline points="7 10 12 15 17 10"/>
        <line x1="12" y1="15" x2="12" y2="3"/>
      </svg>
      <p>{$_('downloads.empty.title')}</p>
      <p class="sub">{$_('downloads.empty.subtitle')}</p>
    </div>
  {:else}
    <div class="history-section">
      <h3>{$_('downloads.history.title')}</h3>
      <div class="download-list">
        {#each downloadItems as item (item.trackId)}
        <div class="download-item">
          {#if item.cover}
            <img class="download-cover" src={item.cover} alt={item.title} loading="lazy" />
          {:else}
            <div class="download-cover" role="img" aria-label="No cover"></div>
          {/if}
          <div class="download-details">
            {#if item.filePath}
              <button 
                class="download-title download-title-btn" 
                type="button" 
                onclick={() => openDownloadedFile(item)}
                aria-label={$_('downloads.actions.showInFolder')}
              >
                {item.title}
              </button>
            {:else}
              <div class="download-title">{item.title}</div>
            {/if}
            <div class="download-sub">
              {#if item.artist}
                <span>{item.artist}</span>
              {/if}
              {#if item.artist && item.album}
                <span class="separator" aria-hidden="true">•</span>
              {/if}
              {#if item.album}
                <span>{item.album}</span>
              {/if}
            </div>
            <div class="progress-container">
              <div class="progress-bar" role="progressbar" aria-valuenow={getProgressWidth(item.percent)} aria-valuemin={0} aria-valuemax={100}>
                <div
                  class="progress-fill {getStatusClass(item.status)}"
                  style="width: {getProgressWidth(item.percent)}%"
                ></div>
              </div>
            </div>
          </div>
          <div class="download-status {getStatusClass(item.status)}">
            {getStatusText(item.status, item.percent)}
            {#if item.status === 'complete' && item.actualQuality}
              <span class="quality-info">
                {#if item.requestedQuality && item.requestedQuality !== item.actualQuality}
                  {$_('downloads.quality.requested')} {formatQualityLabel(item.requestedQuality)} → {formatQualityLabel(item.actualQuality)}
                {:else}
                  {$_('downloads.quality.label')}: {formatQualityLabel(item.actualQuality)}
                {/if}
              </span>
            {/if}
            {#if shouldShowActionButton(item.status)}
              {#if (item.status === 'downloading' || item.status === 'resolving') && item.trackId}
                <button 
                  class="action-btn pause-btn" 
                  type="button" 
                  title={$_('downloads.actions.pause')} 
                  onclick={() => pauseDownload(item)}
                  aria-label={$_('downloads.actions.pause')}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="6" y="4" width="4" height="16"/>
                    <rect x="14" y="4" width="4" height="16"/>
                  </svg>
                </button>
              {:else if item.status === 'paused' && item.track}
                <button 
                  class="action-btn resume-btn" 
                  type="button" 
                  title={$_('downloads.actions.resume')} 
                  onclick={() => resumeDownload(item)}
                  aria-label={$_('downloads.actions.resume')}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polygon points="5 3 19 12 5 21 5 3"/>
                  </svg>
                </button>
              {:else if item.status === 'error' && item.track}
                <button 
                  class="action-btn retry-btn" 
                  type="button" 
                  title={$_('downloads.actions.retry')} 
                  onclick={() => retryDownload(item)}
                  aria-label={$_('downloads.actions.retry')}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="23 4 23 10 17 10"/>
                    <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                  </svg>
                </button>
              {/if}
            {/if}
            {#if item.errorMsg && !['error','paused','downloading','resolving','complete'].includes(item.status)}
              <div class="error-msg" title={item.errorMsg} role="alert">⚠</div>
            {/if}
          </div>
        </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<ExportHistoryModal 
  bind:show={showExportModal}
  history={downloadItems}
  onClose={() => showExportModal = false}
/>

<style>
  .view {
    padding: 28px 32px;
    height: 100%;
    overflow-y: auto;
  }
  
  .header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }

  h2 {
    font-size: 24px;
    font-weight: 700;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .action-btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .action-btn:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .export-btn svg {
    width: 14px;
    height: 14px;
  }

  .clear-btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .clear-btn:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 80px 20px;
    color: var(--text-tertiary);
    text-align: center;
  }
  
  .empty-state p {
    margin-top: 12px;
    font-size: 15px;
  }
  
  .empty-state .sub {
    font-size: 13px;
    margin-top: 4px;
  }

  .history-section h3 {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--text-secondary);
  }
  
  .download-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  
  .download-item {
    display: grid;
    grid-template-columns: 44px 1fr auto;
    gap: 14px;
    align-items: center;
    padding: 12px 16px;
    border-radius: var(--radius);
    background: var(--bg-surface);
  }
  
  .download-cover {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-elevated);
  }
  
  .download-details {
    overflow: hidden;
  }
  
  .download-title {
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .download-title-btn {
    border: 0;
    padding: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .download-title-btn:hover {
    text-decoration: underline;
  }
  
  .download-sub {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .download-sub .separator {
    opacity: 0.5;
  }

  .progress-container {
    margin-top: 8px;
  }
  
  .progress-bar {
    height: 3px;
    background: var(--bg-elevated);
    border-radius: 2px;
    overflow: hidden;
  }
  
  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.3s ease;
  }
  
  .progress-fill.complete {
    background: var(--success);
  }
  
  .progress-fill.error {
    background: var(--error);
  }

  .progress-fill.paused {
    background: var(--text-tertiary);
  }
  
  .download-status {
    font-size: 12px;
    color: var(--text-tertiary);
    white-space: nowrap;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }

  .quality-info {
    font-size: 11px;
    color: var(--text-tertiary);
    margin-left: 2px;
  }

  .download-status.complete {
    color: var(--success);
  }

  .download-status.error {
    color: var(--error);
  }

  .download-status.paused {
    color: var(--text-secondary);
  }

  .download-status .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: transparent;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .download-status .action-btn:hover {
    background: var(--bg-elevated);
  }

  .retry-btn {
    color: var(--error);
  }

  .retry-btn:hover {
    color: var(--text-primary);
  }

  .pause-btn {
    color: var(--text-secondary);
  }

  .pause-btn:hover {
    color: var(--text-primary);
  }

  .resume-btn {
    color: var(--accent);
  }

  .resume-btn:hover {
    color: var(--text-primary);
  }

  .error-msg {
    font-size: 14px;
    cursor: help;
  }
</style>
