<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { downloadHistory, downloads, type DownloadItem } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import QueueView from './QueueView.svelte';
  import ExportHistoryModal from './ExportHistoryModal.svelte';
  import { _ } from 'svelte-i18n';

  interface DownloadProgressEvent {
    track_id: string;
    title: string;
    percent: number;
    status: string;
  }

  let downloadItems: DownloadItem[] = [];
  let showExportModal = false;

  onMount(() => {
    // Subscribe to the download history store
    const unsubHistory = downloadHistory.subscribe(val => {
      downloadItems = val;
    });

    // Listen for download progress events to update the store
    let unlistenProgress: (() => void) | undefined;
    let unlistenTagError: (() => void) | undefined;

    listen<DownloadProgressEvent>('download-progress', (event) => {
      const { track_id, title, percent, status } = event.payload;

      // Update the download history store
      downloadHistory.update(history => {
        const existingIndex = history.findIndex(d => d.trackId === track_id);

        if (existingIndex >= 0) {
          // Only update changed fields to prevent unnecessary reactivity triggers
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
        } else {
          // Defensive: Do not add unrelated unknown items
          return history;
        }
      });
    }).then(fn => {
      unlistenProgress = fn;
    });

    // Listen for tag writing errors
    listen<{track_id: string, title: string, error: string}>('tag-writing-error', (event) => {
      const { track_id, error } = event.payload;
      // Update download history with warning
      downloadHistory.update(history =>
        history.map(item =>
          item.trackId === track_id
            ? { ...item, errorMsg: `Warning: Tags not written - ${error}` }
            : item
        )
      );
    }).then(fn => {
      unlistenTagError = fn;
    });

    // Cleanup event listeners and subscription on unmount
    return () => {
      if (unlistenProgress) {
        unlistenProgress();
      }
      if (unlistenTagError) {
        unlistenTagError();
      }
      unsubHistory();
    };
  });

  function clearHistory() {
    downloadHistory.set([]);
    downloads.update(d => {
      if ('clear' in d && typeof d.clear === "function") d.clear();
      return d;
    });
  }

  function retryDownload(item: DownloadItem) {
    if (!item.track) return;

    // Reset status in download history
    downloadHistory.update(history =>
      history.map(h =>
        h.trackId === item.trackId
          ? { ...h, status: 'downloading', percent: 0, errorMsg: undefined }
          : h
      )
    );

    // Reset downloads map entry so addToQueue won't skip it
    downloads.update(d => {
      if ('delete' in d && typeof d.delete === "function") d.delete(item.trackId);
      return d;
    });

    downloadQueueManager.addToQueue(item.track);
  }

  function pauseDownload(item: DownloadItem) {
    if (!item.trackId) return;
    downloadQueueManager.pauseDownload(item.trackId);
  }

  function resumeDownload(item: DownloadItem) {
    if (!item.trackId) return;
    downloadQueueManager.resumeDownload(item.trackId);
  }

  async function openDownloadedFile(item: DownloadItem) {
    if (!item.filePath) return;

    try {
      await invoke('show_in_folder', { filePath: item.filePath });
    } catch (err) {
      // error already logged
    }
  }

  function getStatusText(status: string, percent: number): string {
    if (status === 'complete') return $_('downloads.status.complete');
    if (status === 'error') return $_('downloads.status.error');
    if (status === 'paused') return $_('downloads.status.paused');
    if (status === 'downloading') return $_('downloads.status.downloading', { values: { percent: Math.round(percent ?? 0) } });
    if (status === 'tagging') return $_('downloads.status.tagging');
    if (status === 'resolving') return $_('downloads.status.resolving');
    return status;
  }

  function formatQualityLabel(value?: string): string {
    if (!value) return '';
    return value.replace('MP3_', 'MP3 ').replace('_', ' ');
  }

  function openExportModal() {
    showExportModal = true;
  }

  function closeExportModal() {
    showExportModal = false;
  }
</script>

<div class="view">
  <div class="header-row">
    <h2>{$_('downloads.title')}</h2>
    {#if downloadItems.length > 0}
      <div class="header-actions">
        <button class="action-btn export-btn" type="button" on:click={openExportModal}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          {$_('downloads.exportHistory')}
        </button>
        <button class="action-btn clear-btn" type="button" on:click={clearHistory}>{$_('downloads.clearHistory')}</button>
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
            <img class="download-cover" src={item.cover} alt="" />
          {:else}
            <div class="download-cover"></div>
          {/if}
          <div class="download-details">
            {#if item.filePath}
              <button class="download-title download-title-btn" type="button" on:click={() => openDownloadedFile(item)}>
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
                <span class="separator">•</span>
              {/if}
              {#if item.album}
                <span>{item.album}</span>
              {/if}
            </div>
            <div class="progress-container">
              <div class="progress-bar">
                <div
                  class="progress-fill {item.status === 'complete' ? 'complete' : ''} {item.status === 'error' ? 'error' : ''} {item.status === 'paused' ? 'paused' : ''}"
                  style="width: {Math.max(0, Math.min(item.percent ?? 0, 100))}%"
                ></div>
              </div>
            </div>
          </div>
          <div class="download-status {item.status === 'complete' ? 'complete' : ''} {item.status === 'error' ? 'error' : ''} {item.status === 'paused' ? 'paused' : ''}">
            {getStatusText(item.status, item.percent)}
            {#if item.status === 'complete' && item.actualQuality}
              <span class="quality-info">
                {#if item.requestedQuality && item.requestedQuality !== item.actualQuality}
                  Requested {formatQualityLabel(item.requestedQuality)} -> Downloaded {formatQualityLabel(item.actualQuality)}
                {:else}
                  Quality: {formatQualityLabel(item.actualQuality)}
                {/if}
              </span>
            {/if}
            {#if (item.status === 'downloading' || item.status === 'resolving') && item.trackId}
              <button class="action-btn pause-btn" type="button" title={$_('downloads.actions.pause')} on:click={() => pauseDownload(item)}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="6" y="4" width="4" height="16"/>
                  <rect x="14" y="4" width="4" height="16"/>
                </svg>
              </button>
            {:else if item.status === 'paused' && item.track}
              <button class="action-btn resume-btn" type="button" title={$_('downloads.actions.resume')} on:click={() => resumeDownload(item)}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polygon points="5 3 19 12 5 21 5 3"/>
                </svg>
              </button>
            {:else if item.status === 'error' && item.track}
              <button class="action-btn retry-btn" type="button" title={$_('downloads.actions.retry')} on:click={() => retryDownload(item)}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="23 4 23 10 17 10"/>
                  <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                </svg>
              </button>
            {/if}
            {#if item.errorMsg && !['error','paused','downloading','resolving','complete'].includes(item.status)}
              <div class="error-msg" title={item.errorMsg}>⚠</div>
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
  onClose={closeExportModal}
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
