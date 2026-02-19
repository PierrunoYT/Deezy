<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { Track } from '$lib/stores';

  interface LyricsData {
    LYRICS_ID?: number;
    LYRICS_TEXT?: string;
    LYRICS_SYNC_JSON?: Array<{
      line: string;
      milliseconds: number;
      duration: number;
    }>;
  }

  interface Props {
    track: Track;
    onClose: () => void;
  }

  let { track, onClose }: Props = $props();

  let lyricsData = $state<LyricsData | null>(null);
  let loading = $state<boolean>(true);
  let error = $state<string>('');
  let useSyncedLyrics = $state<boolean>(true);

  async function loadLyrics() {
    loading = true;
    error = '';
    
    try {
      const result = await invoke<LyricsData>('get_track_lyrics', { 
        trackId: String(track.id) 
      });
      
      lyricsData = result;
      
      if (!result.LYRICS_TEXT && !result.LYRICS_SYNC_JSON) {
        error = 'No lyrics available for this track.';
      }
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function formatLyrics(text: string): string {
    return text.replace(/\n/g, '<br>');
  }

  $effect(() => {
    loadLyrics();
    window.addEventListener('keydown', handleKeydown);
    return () => {
      window.removeEventListener('keydown', handleKeydown);
    };
  });
</script>

<div class="modal-backdrop" onclick={handleBackdropClick}>
  <div class="modal-content">
    <div class="modal-header">
      <div class="track-info">
        <img class="track-cover" src={track.cover_medium} alt="" />
        <div class="track-details">
          <div class="track-title">{track.title}</div>
          <div class="track-artist">{track.artist}</div>
          <div class="track-album">{track.album}</div>
        </div>
      </div>
      <button class="btn-close" onclick={onClose} title="Close (Esc)">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>

    <div class="modal-body">
      {#if loading}
        <div class="lyrics-status">
          <span class="spinner"></span>
          <span>Loading lyrics...</span>
        </div>
      {:else if error}
        <div class="lyrics-status error">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span>{error}</span>
        </div>
      {:else if lyricsData}
        {#if lyricsData.LYRICS_SYNC_JSON && lyricsData.LYRICS_SYNC_JSON.length > 0}
          <div class="lyrics-controls">
            <button 
              class="toggle-btn" 
              class:active={useSyncedLyrics}
              onclick={() => useSyncedLyrics = true}
            >
              Synced
            </button>
            {#if lyricsData.LYRICS_TEXT}
              <button 
                class="toggle-btn" 
                class:active={!useSyncedLyrics}
                onclick={() => useSyncedLyrics = false}
              >
                Plain
              </button>
            {/if}
          </div>
        {/if}

        <div class="lyrics-content">
          {#if useSyncedLyrics && lyricsData.LYRICS_SYNC_JSON && lyricsData.LYRICS_SYNC_JSON.length > 0}
            <div class="synced-lyrics">
              {#each lyricsData.LYRICS_SYNC_JSON as lyricLine}
                <div class="lyric-line">{lyricLine.line}</div>
              {/each}
            </div>
          {:else if lyricsData.LYRICS_TEXT}
            <div class="plain-lyrics">
              {@html formatLyrics(lyricsData.LYRICS_TEXT)}
            </div>
          {:else}
            <div class="lyrics-status">
              <span>No lyrics available</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 20px;
    animation: fadeIn 0.2s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .modal-content {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 100%;
    max-width: 600px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    animation: slideUp 0.3s ease-out;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 24px;
    border-bottom: 1px solid var(--border);
  }

  .track-info {
    display: flex;
    gap: 16px;
    flex: 1;
    min-width: 0;
  }

  .track-cover {
    width: 72px;
    height: 72px;
    border-radius: var(--radius);
    object-fit: cover;
    background: var(--bg-dark);
    flex-shrink: 0;
  }

  .track-details {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
  }

  .track-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-artist {
    font-size: 14px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-album {
    font-size: 13px;
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn-close {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    flex-shrink: 0;
  }

  .btn-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    min-height: 200px;
  }

  .lyrics-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px 20px;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .lyrics-status.error {
    color: var(--error);
  }

  .lyrics-controls {
    display: flex;
    gap: 8px;
    margin-bottom: 20px;
    justify-content: center;
  }

  .toggle-btn {
    padding: 6px 16px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.15s;
  }

  .toggle-btn:hover {
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .toggle-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .lyrics-content {
    line-height: 1.8;
    font-size: 15px;
    color: var(--text-primary);
  }

  .plain-lyrics {
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  .synced-lyrics {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .lyric-line {
    padding: 8px 0;
    transition: color 0.2s;
  }

  .lyric-line:empty {
    height: 12px;
  }
</style>
