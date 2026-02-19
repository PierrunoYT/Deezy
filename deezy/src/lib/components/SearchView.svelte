<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { loggedIn, downloads, type Track } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import { searchRateLimiter } from '$lib/rateLimiter';

  interface AlbumResult {
    id: number;
    title: string;
    artist: string;
    artist_id: number;
    cover_small: string;
    cover_medium: string;
    nb_tracks: number;
  }

  interface ArtistResult {
    id: number;
    name: string;
    picture_small: string;
    picture_medium: string;
    nb_album: number;
    nb_fan: number;
  }

  interface SelectedArtist {
    id: number;
    name: string;
    picture: string;
  }

  type SearchType = 'tracks' | 'albums' | 'artists';

  let searchQuery = $state<string>('');
  let searchType = $state<SearchType>('tracks');
  let results = $state<Track[]>([]);
  let albumResults = $state<AlbumResult[]>([]);
  let artistResults = $state<ArtistResult[]>([]);
  let searching = $state<boolean>(false);
  let errorMsg = $state<string>('');
  let isLoggedIn = $state<boolean>(false);
  let downloadStates = $state<Map<string, string>>(new Map());
  let downloadingAlbums = $state<Set<number>>(new Set());

  // Artist discography state
  let selectedArtist = $state<SelectedArtist | null>(null);
  let artistAlbums = $state<AlbumResult[]>([]);
  let loadingDiscography = $state<boolean>(false);
  let discographyError = $state<string>('');

  let searchTimeout: ReturnType<typeof setTimeout> | undefined;

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
      artistResults = [];
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
    artistResults = [];
    errorMsg = '';
    selectedArtist = null;
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
    artistResults = [];

    try {
      await searchRateLimiter.throttle();

      if (searchType === 'tracks') {
        const data = await invoke<Track[]>('search_tracks', { query: searchQuery.trim() });
        results = data;
        if (results.length === 0) errorMsg = 'No results found.';
      } else if (searchType === 'albums') {
        const data = await invoke<AlbumResult[]>('search_albums', { query: searchQuery.trim() });
        albumResults = data;
        if (albumResults.length === 0) errorMsg = 'No results found.';
      } else {
        const data = await invoke<ArtistResult[]>('search_artists', { query: searchQuery.trim() });
        artistResults = data;
        if (artistResults.length === 0) errorMsg = 'No results found.';
      }
    } catch (err) {
      errorMsg = String(err);
    } finally {
      searching = false;
    }
  }

  async function openArtist(id: number, name: string, picture: string) {
    selectedArtist = { id, name, picture };
    artistAlbums = [];
    discographyError = '';
    loadingDiscography = true;

    try {
      const data = await invoke<AlbumResult[]>('get_artist_albums', { artistId: String(id) });
      artistAlbums = data;
      if (artistAlbums.length === 0) discographyError = 'No albums found for this artist.';
    } catch (err) {
      discographyError = String(err);
    } finally {
      loadingDiscography = false;
    }
  }

  function closeArtist() {
    selectedArtist = null;
    artistAlbums = [];
    discographyError = '';
  }

  async function downloadTrack(track: Track) {
    const trackId = String(track.id);
    const state = downloadStates.get(trackId);
    if (state === 'downloading' || state === 'complete') return;
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

  function formatFans(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M fans`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K fans`;
    return `${n} fans`;
  }
</script>

<div class="view">
  <div class="search-header">
    {#if selectedArtist}
      <div class="artist-page-header">
        <button class="btn-back" onclick={closeArtist}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          Back
        </button>
        <div class="artist-hero">
          {#if selectedArtist.picture}
            <img class="artist-hero-img" src={selectedArtist.picture} alt={selectedArtist.name} />
          {:else}
            <div class="artist-hero-placeholder">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <circle cx="12" cy="8" r="4"/><path d="M4 20c0-4 3.6-7 8-7s8 3 8 7"/>
              </svg>
            </div>
          {/if}
          <div class="artist-hero-info">
            <div class="artist-hero-name">{selectedArtist.name}</div>
            <div class="artist-hero-meta">Discography</div>
          </div>
        </div>
      </div>
    {:else}
      <div class="search-tabs">
        <button class="tab-btn" class:active={searchType === 'tracks'} onclick={() => switchSearchType('tracks')}>Tracks</button>
        <button class="tab-btn" class:active={searchType === 'albums'} onclick={() => switchSearchType('albums')}>Albums</button>
        <button class="tab-btn" class:active={searchType === 'artists'} onclick={() => switchSearchType('artists')}>Artists</button>
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
    {/if}
  </div>

  <!-- Artist discography view -->
  {#if selectedArtist}
    {#if loadingDiscography}
      <div class="status-message info"><span class="spinner"></span> Loading discography...</div>
    {:else if discographyError}
      <div class="status-message error">{discographyError}</div>
    {:else if artistAlbums.length > 0}
      <div class="results-list">
        {#each artistAlbums as album (album.id)}
          <div class="album-item">
            <img class="album-cover" src={album.cover_medium} alt="" loading="lazy" />
            <div class="album-info">
              <div class="album-title">{album.title}</div>
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

  <!-- Normal search results -->
  {:else}
    {#if searching}
      <div class="status-message info"><span class="spinner"></span> Searching...</div>
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
            <img class="track-cover" src={track.cover_small} alt="" loading="lazy" />
            <div class="track-info">
              <div class="track-title">{track.title}</div>
              <button
                class="track-artist artist-link"
                onclick={() => openArtist(track.artist_id, track.artist, '')}
                title="Browse {track.artist}'s discography"
              >{track.artist}</button>
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
            <img class="album-cover" src={album.cover_medium} alt="" loading="lazy" />
            <div class="album-info">
              <div class="album-title">{album.title}</div>
              <button
                class="album-artist artist-link"
                onclick={() => openArtist(album.artist_id, album.artist, '')}
                title="Browse {album.artist}'s discography"
              >{album.artist}</button>
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

    {#if searchType === 'artists' && artistResults.length > 0}
      <div class="artist-grid">
        {#each artistResults as artist (artist.id)}
          <button class="artist-card" onclick={() => openArtist(artist.id, artist.name, artist.picture_medium)}>
            {#if artist.picture_medium}
              <img class="artist-card-img" src={artist.picture_medium} alt={artist.name} loading="lazy" />
            {:else}
              <div class="artist-card-placeholder">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                  <circle cx="12" cy="8" r="4"/><path d="M4 20c0-4 3.6-7 8-7s8 3 8 7"/>
                </svg>
              </div>
            {/if}
            <div class="artist-card-name">{artist.name}</div>
            <div class="artist-card-meta">
              {artist.nb_album} album{artist.nb_album !== 1 ? 's' : ''} · {formatFans(artist.nb_fan)}
            </div>
          </button>
        {/each}
      </div>
    {/if}
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

  /* Artist page header */
  .artist-page-header {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .btn-back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px 6px 10px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.15s;
    align-self: flex-start;
  }

  .btn-back:hover {
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .artist-hero {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .artist-hero-img {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    object-fit: cover;
    background: var(--bg-elevated);
  }

  .artist-hero-placeholder {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
  }

  .artist-hero-name {
    font-size: 22px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .artist-hero-meta {
    font-size: 13px;
    color: var(--text-tertiary);
    margin-top: 2px;
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

  .artist-link {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    width: 100%;
    color: var(--text-secondary);
    font-size: 13px;
    transition: color 0.15s;
  }

  .artist-link:hover {
    color: var(--accent);
    text-decoration: underline;
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

  /* Artist search results grid */
  .artist-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 16px;
    padding: 4px 0;
  }

  .artist-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 16px 12px;
    border-radius: var(--radius);
    border: 1px solid transparent;
    background: var(--bg-elevated);
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
    font-family: inherit;
  }

  .artist-card:hover {
    background: var(--bg-hover);
    border-color: var(--border);
    transform: translateY(-2px);
  }

  .artist-card-img {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    object-fit: cover;
    background: var(--bg-dark);
  }

  .artist-card-placeholder {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    background: var(--bg-dark);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
  }

  .artist-card-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
  }

  .artist-card-meta {
    font-size: 12px;
    color: var(--text-tertiary);
  }
</style>
