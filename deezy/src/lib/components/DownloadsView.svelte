<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { downloadHistory, downloads, type DownloadItem } from '$lib/stores';

  interface DownloadProgressEvent {
    track_id: string;
    title: string;
    percent: number;
    status: string;
  }

  let downloadItems = $state<DownloadItem[]>([]);

  // Subscribe to the download history store using idiomatic Svelte 5 pattern
  $effect(() => {
    try {
      const unsubscribe = downloadHistory.subscribe(val => {
        downloadItems = val;
      });
      return unsubscribe;
    } catch (err) {
      console.error('Error in effect:', err);
    }
  });

  onMount(() => {
    // Listen for download progress events to update the store
    let unlistenProgress: (() => void) | undefined;
    let unlistenTagError: (() => void) | undefined;

    listen<DownloadProgressEvent>('download-progress', (event) => {
      const { track_id, title, percent, status } = event.payload;

      console.log('Download progress event:', { track_id, title, percent, status });

      // Update the download history store
      downloadHistory.update(history => {
        const existingIndex = history.findIndex(d => d.trackId === track_id);

        if (existingIndex >= 0) {
          // Update existing item
          return history.map((item, idx) =>
            idx === existingIndex
              ? { ...item, title, percent, status }
              : item
          );
        } else {
          // Add new item if it doesn't exist (shouldn't happen normally)
          return [{
            trackId: track_id,
            title: title,
            artist: '',
            album: '',
            cover: '',
            percent: percent,
            status: status
          }, ...history];
        }
      });

      console.log('Updated downloadHistory');
    }).then(fn => {
      unlistenProgress = fn;
    });

    // Listen for tag writing errors
    listen<{track_id: string, title: string, error: string}>('tag-writing-error', (event) => {
      const { track_id, title, error } = event.payload;
      console.warn('Tag writing failed for', title, ':', error);

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

    // Cleanup event listeners on unmount
    return () => {
      if (unlistenProgress) {
        unlistenProgress();
      }
      if (unlistenTagError) {
        unlistenTagError();
      }
    };
  });

  function clearHistory() {
    downloadHistory.set([]);
    downloads.update(d => {
      d.clear();
      return d;
    });
  }

  function getStatusText(status: string, percent: number): string {
    if (status === 'complete') return 'Done';
    if (status === 'error') return 'Error';
    if (status === 'downloading') return `${Math.round(percent)}%`;
    if (status === 'tagging') return 'Writing tags...';
    if (status === 'resolving') return 'Resolving...';
    return status;
  }
</script>

<div class="view">
  <div class="header-row">
    <h2>Downloads</h2>
    {#if downloadItems.length > 0}
      <button class="clear-btn" onclick={clearHistory}>Clear history</button>
    {/if}
  </div>
  
  {#if downloadItems.length === 0}
    <div class="empty-state">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
        <polyline points="7 10 12 15 17 10"/>
        <line x1="12" y1="15" x2="12" y2="3"/>
      </svg>
      <p>No downloads yet</p>
      <p class="sub">Search for tracks and click the download button</p>
    </div>
  {:else}
    <div class="download-list">
      {#each downloadItems as item (item.trackId)}
        <div class="download-item">
          {#if item.cover}
            <img class="download-cover" src={item.cover} alt="" />
          {:else}
            <div class="download-cover"></div>
          {/if}
          <div class="download-details">
            <div class="download-title">{item.title}</div>
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
                  class="progress-fill {item.status === 'complete' ? 'complete' : ''} {item.status === 'error' ? 'error' : ''}"
                  style="width: {item.percent}%"
                ></div>
              </div>
            </div>
          </div>
          <div class="download-status {item.status === 'complete' ? 'complete' : ''} {item.status === 'error' ? 'error' : ''}">
            {getStatusText(item.status, item.percent)}
            {#if item.errorMsg}
              <div class="error-msg" title={item.errorMsg}>⚠</div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

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
  
  .download-status {
    font-size: 12px;
    color: var(--text-tertiary);
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .download-status.complete {
    color: var(--success);
  }

  .download-status.error {
    color: var(--error);
  }

  .error-msg {
    font-size: 14px;
    cursor: help;
  }
</style>
