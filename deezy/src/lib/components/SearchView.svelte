<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { loggedIn, downloads, type Track } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import { searchRateLimiter } from '$lib/rateLimiter';

  interface AlbumResult {
    id: number;
    title: string;
    artist: string;
    cover_small: string;
    cover_medium: string;
    nb_tracks: number;
  }

  type SearchType = 'tracks' | 'albums';

  let searchQuery = $state<string>('');
  let searchType = $state<SearchType>('tracks');
  let results = $state<Track[]>([]);
  let albumResults = $state<AlbumResult[]>([]);
  let searching = $state<boolean>(false);
  let errorMsg = $state<string>('');
  let isLoggedIn = $state<boolean>(false);
  let downloadStates = $state<Map<string, string>>(new Map());
  let downloadingAlbums = $state<Set<number>>(new Set());

  let searchTimeout: ReturnType<typeof setTimeout> | undefined;

  // Use idiomatic Svelte 5 pattern with proper cleanup
  $effect(() => {
    try {
      const unsubscribe1 = loggedIn.subscribe(val => isLoggedIn = val);
      const unsubscribe2 = downloads.subscribe(val => downloadStates = val);
      return () => {
        unsubscribe1();
        unsubscribe2();
      };
    } catch (err) {
      console.error('Error in effect:', err);
    }
  });
  
  function handleInput() {
    clearTimeout(searchTimeout);
    errorMsg = '';
    
    if (searchQuery.trim().length < 2) {
      results = [];
      albumResults = [];
      return;
    }
    
    searchTimeout = setTimeout(() => doSearch(), 400);
  }
  
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      clearTimeout(searchTimeout);
      if (searchQuery.trim()) doSearch();
    }
  }

  function switchSearchType(type: SearchType) {
    searchType = type;
    results = [];
    albumResults = [];
    errorMsg = '';
    if (searchQuery.trim().length >= 2) {
      doSearch();
    }
  }
  
  async function doSearch() {
    if (!isLoggedIn) {
      errorMsg = 'Please set your ARL token in Settings first.';
      return;
    }

    searching = true;
    errorMsg = '';
    results = [];
    albumResults = [];

    try {
      await searchRateLimiter.throttle();

      if (searchType === 'tracks') {
        const data = await invoke<Track[]>('search_tracks', { query: searchQuery.trim() });
        results = data;
        if (results.length === 0) {
          errorMsg = 'No results found.';
        }
      } else {
        const data = await invoke<AlbumResult[]>('search_albums', { query: searchQuery.trim() });
        albumResults = data;
        if (albumResults.length === 0) {
          errorMsg = 'No results found.';
        }
      }
    } catch (err) {
      errorMsg = String(err);
    } finally {
      searching = false;
    }
  }

  async function downloadTrack(track: Track) {
    const trackId = String(track.id);
    const state = downloadStates.get(trackId);

    console.log('Download clicked for track:', trackId, 'Current state:', state);

    if (state === 'downloading' || state === 'complete') {
      console.log('Download already in progress or complete, skipping');
      return;
    }

    await downloadQueueManager.addToQueue(track);
  }

  async function downloadAlbum(album: AlbumResult) {
    if (downloadingAlbums.has(album.id)) return;

    downloadingAlbums = new Set([...downloadingAlbums, album.id]);

    try {
      const tracks = await invoke<Track[]>('get_album_tracks', { albumId: String(album.id) });
      for (const track of tracks) {
        await downloadQueueManager.addToQueue(track);
      }
    } catch (err) {
      errorMsg = `Failed to get album tracks: ${String(err)}`;
    } finally {
      downloadingAlbums = new Set([...downloadingAlbums].filter(id => id !== album.id));
    }
  }
  
  function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = String(seconds % 60).padStart(2, '0');
    return `${mins}:${secs}`;
  }
</script>

<div class="view">
  <div class="search-header">
    <div class="search-tabs">
      <button
        class="tab-btn"
        class:active={searchType === 'tracks'}
        onclick={() => switchSearchType('tracks')}
      >Tracks</button>
      <button
        class="tab-btn"
        class:active={searchType === 'albums'}
        onclick={() => switchSearchType('albums')}
      >Albums</button>
    </div>
    <div class="search-bar">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <input 
        type="text" 
        bind:value={searchQuery}
        oninput={handleInput}
        onkeydown={handleKeydown}
        placeholder="Search for {searchType}..." 
        autocomplete="off" 
      />
    </div>
  </div>
  
  {#if searching}
    <div class="status-message info">
      <span class="spinner"></span> Searching...
    </div>
  {:else if errorMsg}
    <div class="status-message error">{errorMsg}</div>
  {/if}
  
  {#if searchType === 'tracks' && results.length > 0}
    <div class="results-header">
      <span class="col-title">Title</span>
      <span class="col-album">Album</span>
      <span class="col-duration">Duration</span>
      <span class="col-action"></span>
    </div>
    
    <div class="results-list">
      {#each results as track (track.id)}
        <div class="track-item">
          <img 
            class="track-cover" 
            src={track.cover_small} 
            alt="" 
            loading="lazy"
          />
          <div class="track-info">
            <div class="track-title">{track.title}</div>
            <div class="track-artist">{track.artist}</div>
          </div>
          <div class="track-album">{track.album}</div>
          <div class="track-duration">{formatDuration(track.duration)}</div>
          <button 
            class="btn-download {downloadStates.get(String(track.id)) === 'downloading' ? 'downloading' : ''} {downloadStates.get(String(track.id)) === 'complete' ? 'done' : ''}"
            onclick={() => downloadTrack(track)}
            title="Download"
          >
            {#if downloadStates.get(String(track.id)) === 'downloading'}
              <span class="spinner"></span>
            {:else if downloadStates.get(String(track.id)) === 'complete'}
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            {:else}
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            {/if}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if searchType === 'albums' && albumResults.length > 0}
    <div class="results-list">
      {#each albumResults as album (album.id)}
        <div class="album-item">
          <img 
            class="album-cover" 
            src={album.cover_medium} 
            alt="" 
            loading="lazy"
          />
          <div class="album-info">
            <div class="album-title">{album.title}</div>
            <div class="album-artist">{album.artist}</div>
            <div class="album-meta">{album.nb_tracks} track{album.nb_tracks !== 1 ? 's' : ''}</div>
          </div>
          <button 
            class="btn-download-all"
            class:downloading={downloadingAlbums.has(album.id)}
            onclick={() => downloadAlbum(album)}
            disabled={downloadingAlbums.has(album.id)}
          >
            {#if downloadingAlbums.has(album.id)}
              <span class="spinner"></span> Adding...
            {:else}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
              Download All
            {/if}
          </button>
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
  
  .search-header {
    position: sticky;
    top: 0;
    background: var(--bg-dark);
    padding-bottom: 20px;
    z-index: 10;
  }
  
  .search-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 12px;
  }
  
  .tab-btn {
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
  
  .tab-btn:hover {
    border-color: var(--accent);
    color: var(--text-primary);
  }
  
  .tab-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  
  .search-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 24px;
    padding: 12px 20px;
    transition: border-color 0.2s;
  }
  
  .search-bar:focus-within {
    border-color: var(--accent);
  }
  
  .search-bar svg {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }
  
  .search-bar input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 15px;
    outline: none;
    font-family: inherit;
  }
  
  .search-bar input::placeholder {
    color: var(--text-tertiary);
  }
  
  .status-message {
    margin-top: 16px;
    padding: 10px 14px;
    border-radius: var(--radius);
    font-size: 13px;
  }
  
  .status-message.info {
    background: var(--accent-dim);
    color: var(--accent);
    border: 1px solid rgba(162, 56, 255, 0.2);
  }
  
  .status-message.error {
    background: rgba(231, 76, 60, 0.1);
    color: var(--error);
    border: 1px solid rgba(231, 76, 60, 0.2);
  }
  
  .results-header {
    display: grid;
    grid-template-columns: 52px 1fr 160px 80px 48px;
    gap: 12px;
    padding: 8px 16px;
    color: var(--text-tertiary);
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  
  .results-header .col-title {
    grid-column: 2;
  }
  
  .results-list {
    display: flex;
    flex-direction: column;
  }
  
  .track-item {
    display: grid;
    grid-template-columns: 52px 1fr 160px 80px 48px;
    gap: 12px;
    align-items: center;
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    cursor: default;
    transition: background 0.15s;
  }
  
  .track-item:hover {
    background: var(--bg-hover);
  }
  
  .track-cover {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-elevated);
  }
  
  .track-info {
    overflow: hidden;
  }
  
  .track-title {
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .track-artist {
    font-size: 13px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .track-album {
    font-size: 13px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .track-duration {
    font-size: 13px;
    color: var(--text-tertiary);
    text-align: right;
  }
  
  .btn-download {
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
  }
  
  .btn-download:hover {
    background: var(--accent);
    color: white;
    transform: scale(1.1);
  }
  
  .btn-download.downloading {
    color: var(--accent);
    animation: pulse 1.5s infinite;
  }
  
  .btn-download.done {
    color: var(--success);
  }
  
  .album-item {
    display: grid;
    grid-template-columns: 72px 1fr auto;
    gap: 16px;
    align-items: center;
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    cursor: default;
    transition: background 0.15s;
  }
  
  .album-item:hover {
    background: var(--bg-hover);
  }
  
  .album-cover {
    width: 64px;
    height: 64px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-elevated);
  }
  
  .album-info {
    overflow: hidden;
  }
  
  .album-title {
    font-size: 15px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .album-artist {
    font-size: 13px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .album-meta {
    font-size: 12px;
    color: var(--text-tertiary);
    margin-top: 2px;
  }
  
  .btn-download-all {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: 20px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  
  .btn-download-all:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  
  .btn-download-all.downloading {
    color: var(--accent);
    border-color: var(--accent);
    cursor: wait;
  }
  
  .btn-download-all:disabled {
    opacity: 0.7;
  }
</style>
