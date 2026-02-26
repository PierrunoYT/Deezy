<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { loggedIn, downloads, searchHistory, audioPlayer, type Track } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import { searchRateLimiter } from '$lib/rateLimiter';
  import { onMount } from 'svelte';
  import { keyboardShortcuts } from '$lib/keyboardShortcuts';
  import { audioPlayerManager } from '$lib/audioPlayer';
  import LyricsModal from './LyricsModal.svelte';
  import { _ } from 'svelte-i18n';
  import { formatDuration, formatFans } from '$lib/i18n/formatters';

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

  interface PlaylistResult {
    id: number;
    title: string;
    creator: string;
    cover_small: string;
    cover_medium: string;
    nb_tracks: number;
  }

  interface SelectedArtist {
    id: number;
    name: string;
    picture: string;
  }

  interface SelectedPlaylist {
    id: number;
    title: string;
    cover: string;
    creator: string;
  }

  type SearchType = 'tracks' | 'albums' | 'artists' | 'playlists';

  let searchQuery = $state<string>('');
  let searchType = $state<SearchType>('tracks');
  let results = $state<Track[]>([]);
  let albumResults = $state<AlbumResult[]>([]);
  let artistResults = $state<ArtistResult[]>([]);
  let playlistResults = $state<PlaylistResult[]>([]);
  let searching = $state<boolean>(false);
  let errorMsg = $state<string>('');
  let isLoggedIn = $state<boolean>(false);
  let downloadStates = $state<Map<string, string>>(new Map());
  let downloadingAlbums = $state<Set<number>>(new Set());
  let downloadingPlaylists = $state<Set<number>>(new Set());
  let showSearchHistory = $state<boolean>(false);
  let history = $state<string[]>([]);
  let searchInputRef = $state<HTMLInputElement | undefined>(undefined);

  // Artist discography state
  let selectedArtist = $state<SelectedArtist | null>(null);
  let artistAlbums = $state<AlbumResult[]>([]);
  let loadingDiscography = $state<boolean>(false);
  let discographyError = $state<string>('');

  // Playlist detail state
  let selectedPlaylist = $state<SelectedPlaylist | null>(null);
  let playlistTracks = $state<Track[]>([]);
  let loadingPlaylist = $state<boolean>(false);
  let playlistError = $state<string>('');

  // Lyrics modal state
  let lyricsTrack = $state<Track | null>(null);

  // Audio player state
  let currentPlayingTrack = $state<Track | null>(null);
  let isPlaying = $state<boolean>(false);

  let searchTimeout: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    try {
      const unsubscribe1 = loggedIn.subscribe(val => isLoggedIn = val);
      const unsubscribe2 = downloads.subscribe(val => downloadStates = val);
      const unsubscribe3 = searchHistory.subscribe(val => history = val);
      const unsubscribe4 = audioPlayer.subscribe(state => {
        currentPlayingTrack = state.currentTrack;
        isPlaying = state.isPlaying;
      });
      return () => {
        unsubscribe1();
        unsubscribe2();
        unsubscribe3();
        unsubscribe4();
      };
    } catch (err) {
      console.error('Error subscribing to stores:', err);
    }
  });

  onMount(() => {
    loadSearchHistory();
    
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest('.search-bar') && !target.closest('.search-history-dropdown')) {
        showSearchHistory = false;
      }
    };
    
    document.addEventListener('click', handleClickOutside);

    keyboardShortcuts.register('focus-search', {
      key: 'f',
      ctrl: true,
      description: 'Focus search input',
      category: 'search',
      action: () => {
        searchInputRef?.focus();
        searchInputRef?.select();
      }
    });

    keyboardShortcuts.register('clear-search', {
      key: 'Escape',
      description: 'Clear search / Go back',
      category: 'search',
      action: handleEscapeAction
    });

    return () => {
      document.removeEventListener('click', handleClickOutside);
      keyboardShortcuts.unregister('focus-search');
      keyboardShortcuts.unregister('clear-search');
      clearTimeout(searchTimeout);
    };
  });

  function handleEscapeAction(): void {
    if (selectedPlaylist) {
      closePlaylist();
    } else if (selectedArtist) {
      closeArtist();
    } else if (lyricsTrack) {
      closeLyrics();
    } else if (searchQuery) {
      clearSearch();
    }
  }

  function clearSearch(): void {
    searchQuery = '';
    results = [];
    albumResults = [];
    artistResults = [];
    playlistResults = [];
    errorMsg = '';
  }

  async function loadSearchHistory(): Promise<void> {
    try {
      const data = await invoke<string[]>('get_search_history');
      searchHistory.set(data);
    } catch (err) {
      console.error('Failed to load search history:', err);
    }
  }

  async function addToSearchHistory(query: string): Promise<void> {
    try {
      await invoke('add_search_history', { query });
      await loadSearchHistory();
    } catch (err) {
      console.error('Failed to add to search history:', err);
    }
  }

  function resetResults(): void {
    results = [];
    albumResults = [];
    artistResults = [];
    playlistResults = [];
  }
  
  function handleInput(): void {
    clearTimeout(searchTimeout);
    errorMsg = '';
    showSearchHistory = false;
    
    if (searchQuery.trim().length < 2) {
      resetResults();
      return;
    }
    
    searchTimeout = setTimeout(() => doSearch(), 400);
  }
  
  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter') {
      clearTimeout(searchTimeout);
      if (searchQuery.trim()) doSearch();
    } else if (e.key === 'Escape') {
      showSearchHistory = false;
    }
  }

  function handleFocus(): void {
    if (searchQuery.trim().length === 0 && history.length > 0) {
      showSearchHistory = true;
    }
  }

  function selectHistoryItem(item: string): void {
    searchQuery = item;
    showSearchHistory = false;
    doSearch();
  }

  function switchSearchType(type: SearchType): void {
    searchType = type;
    resetResults();
    errorMsg = '';
    selectedArtist = null;
    selectedPlaylist = null;
    if (searchQuery.trim().length >= 2) {
      doSearch();
    }
  }
  
  async function doSearch(): Promise<void> {
    if (!isLoggedIn) {
      errorMsg = $_('search.status.loginRequired');
      return;
    }

    const query = searchQuery.trim();
    if (!query) return;

    searching = true;
    errorMsg = '';
    resetResults();
    showSearchHistory = false;

    try {
      await searchRateLimiter.throttle();

      const searchHandlers = {
        tracks: async () => {
          results = await invoke<Track[]>('search_tracks', { query });
          return results.length;
        },
        albums: async () => {
          albumResults = await invoke<AlbumResult[]>('search_albums', { query });
          return albumResults.length;
        },
        artists: async () => {
          artistResults = await invoke<ArtistResult[]>('search_artists', { query });
          return artistResults.length;
        },
        playlists: async () => {
          playlistResults = await invoke<PlaylistResult[]>('search_playlists', { query });
          return playlistResults.length;
        }
      };

      const resultCount = await searchHandlers[searchType]();
      
      if (resultCount === 0) {
        errorMsg = $_('search.status.noResults');
      } else {
        await addToSearchHistory(query);
      }
    } catch (err) {
      errorMsg = String(err);
    } finally {
      searching = false;
    }
  }

  async function openArtist(id: number, name: string, picture: string): Promise<void> {
    selectedArtist = { id, name, picture };
    artistAlbums = [];
    discographyError = '';
    loadingDiscography = true;

    try {
      const data = await invoke<AlbumResult[]>('get_artist_albums', { artistId: String(id) });
      artistAlbums = data;
      if (artistAlbums.length === 0) {
        discographyError = $_('search.artist.noAlbums');
      }
    } catch (err) {
      discographyError = String(err);
    } finally {
      loadingDiscography = false;
    }
  }

  function closeArtist(): void {
    selectedArtist = null;
    artistAlbums = [];
    discographyError = '';
  }

  async function openPlaylist(playlist: PlaylistResult): Promise<void> {
    selectedPlaylist = { 
      id: playlist.id, 
      title: playlist.title, 
      cover: playlist.cover_medium, 
      creator: playlist.creator 
    };
    playlistTracks = [];
    playlistError = '';
    loadingPlaylist = true;

    try {
      const data = await invoke<Track[]>('get_playlist_tracks', { playlistId: String(playlist.id) });
      playlistTracks = data;
      if (playlistTracks.length === 0) {
        playlistError = $_('search.playlist.noTracks');
      }
    } catch (err) {
      playlistError = String(err);
    } finally {
      loadingPlaylist = false;
    }
  }

  function closePlaylist(): void {
    selectedPlaylist = null;
    playlistTracks = [];
    playlistError = '';
  }

  async function downloadPlaylist(playlist: SelectedPlaylist): Promise<void> {
    if (downloadingPlaylists.has(playlist.id)) return;
    downloadingPlaylists = new Set([...downloadingPlaylists, playlist.id]);

    try {
      let tracks = playlistTracks;
      if (tracks.length === 0) {
        tracks = await invoke<Track[]>('get_playlist_tracks', { playlistId: String(playlist.id) });
      }
      for (const track of tracks) {
        await downloadQueueManager.addToQueue(track);
      }
    } catch (err) {
      errorMsg = $_('search.playlist.downloadError', { values: { error: String(err) } });
    } finally {
      downloadingPlaylists = new Set([...downloadingPlaylists].filter(id => id !== playlist.id));
    }
  }

  async function downloadPlaylistFromResult(playlist: PlaylistResult): Promise<void> {
    if (downloadingPlaylists.has(playlist.id)) return;
    downloadingPlaylists = new Set([...downloadingPlaylists, playlist.id]);

    try {
      const tracks = await invoke<Track[]>('get_playlist_tracks', { playlistId: String(playlist.id) });
      for (const track of tracks) {
        await downloadQueueManager.addToQueue(track);
      }
    } catch (err) {
      errorMsg = $_('search.playlist.downloadError', { values: { error: String(err) } });
    } finally {
      downloadingPlaylists = new Set([...downloadingPlaylists].filter(id => id !== playlist.id));
    }
  }

  async function downloadTrack(track: Track): Promise<void> {
    const trackId = String(track.id);
    const state = downloadStates.get(trackId);
    if (state === 'downloading' || state === 'complete') return;
    await downloadQueueManager.addToQueue(track);
  }

  async function downloadAlbum(album: AlbumResult): Promise<void> {
    if (downloadingAlbums.has(album.id)) return;
    downloadingAlbums = new Set([...downloadingAlbums, album.id]);

    try {
      const tracks = await invoke<Track[]>('get_album_tracks', { albumId: String(album.id) });
      for (const track of tracks) {
        await downloadQueueManager.addToQueue(track);
      }
    } catch (err) {
      errorMsg = $_('search.album.downloadError', { values: { error: String(err) } });
    } finally {
      downloadingAlbums = new Set([...downloadingAlbums].filter(id => id !== album.id));
    }
  }

  function openLyrics(track: Track): void {
    lyricsTrack = track;
  }

  function closeLyrics(): void {
    lyricsTrack = null;
  }

  function playTrack(track: Track): void {
    audioPlayerManager.play(track);
  }

  function isTrackPlaying(track: Track): boolean {
    return currentPlayingTrack?.id === track.id && isPlaying;
  }

  function getDownloadButtonState(trackId: string): 'idle' | 'downloading' | 'complete' {
    const state = downloadStates.get(trackId);
    if (state === 'downloading') return 'downloading';
    if (state === 'complete') return 'complete';
    return 'idle';
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
          {$_('search.artist.back')}
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
            <div class="artist-hero-meta">{$_('search.artist.discography')}</div>
          </div>
        </div>
      </div>
    {:else if selectedPlaylist}
      <div class="artist-page-header">
        <button class="btn-back" onclick={closePlaylist}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          {$_('search.playlist.back')}
        </button>
        <div class="artist-hero">
          {#if selectedPlaylist.cover}
            <img class="artist-hero-img playlist-cover" src={selectedPlaylist.cover} alt={selectedPlaylist.title} />
          {:else}
            <div class="artist-hero-placeholder">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>
              </svg>
            </div>
          {/if}
          <div class="artist-hero-info">
            <div class="artist-hero-name">{selectedPlaylist.title}</div>
            <div class="artist-hero-meta">{$_('search.playlist.by', { values: { creator: selectedPlaylist.creator } })}</div>
          </div>
        </div>
      </div>
    {:else}
      <div class="search-tabs">
        <button class="tab-btn" class:active={searchType === 'tracks'} onclick={() => switchSearchType('tracks')}>{$_('search.tabs.tracks')}</button>
        <button class="tab-btn" class:active={searchType === 'albums'} onclick={() => switchSearchType('albums')}>{$_('search.tabs.albums')}</button>
        <button class="tab-btn" class:active={searchType === 'artists'} onclick={() => switchSearchType('artists')}>{$_('search.tabs.artists')}</button>
        <button class="tab-btn" class:active={searchType === 'playlists'} onclick={() => switchSearchType('playlists')}>{$_('search.tabs.playlists')}</button>
      </div>
      <div class="search-bar-container">
        <div class="search-bar">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <input 
            type="text" 
            bind:this={searchInputRef}
            bind:value={searchQuery}
            oninput={handleInput}
            onkeydown={handleKeydown}
            onfocus={handleFocus}
            placeholder={$_(`search.placeholder.${searchType}`)}
            autocomplete="off" 
          />
        </div>
        
        {#if showSearchHistory && history.length > 0}
          <div class="search-history-dropdown">
            <div class="search-history-header">
              <span>{$_('search.history.title')}</span>
            </div>
            {#each history as item (item)}
              <button class="search-history-item" onclick={() => selectHistoryItem(item)}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
                </svg>
                <span>{item}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Artist discography view -->
  {#if selectedArtist}
    {#if loadingDiscography}
      <div class="status-message info"><span class="spinner"></span> {$_('search.artist.loadingDiscography')}</div>
    {:else if discographyError}
      <div class="status-message error">{discographyError}</div>
    {:else if artistAlbums.length > 0}
      <div class="results-list">
        {#each artistAlbums as album (album.id)}
          <div class="album-item">
            <img class="album-cover" src={album.cover_medium} alt="" loading="lazy" />
            <div class="album-info">
              <div class="album-title">{album.title}</div>
              {#if album.nb_tracks > 0}
                <div class="album-meta">{$_('search.album.tracks', { values: { count: album.nb_tracks } })}</div>
              {/if}
            </div>
            <button 
              class="btn-download-all"
              class:downloading={downloadingAlbums.has(album.id)}
              onclick={() => downloadAlbum(album)}
              disabled={downloadingAlbums.has(album.id)}
            >
              {#if downloadingAlbums.has(album.id)}
                <span class="spinner"></span> {$_('search.album.adding')}
              {:else}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                  <polyline points="7 10 12 15 17 10"/>
                  <line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                {$_('search.album.downloadAll')}
              {/if}
            </button>
          </div>
        {/each}
      </div>
    {/if}

  <!-- Playlist detail view -->
  {:else if selectedPlaylist}
    <div class="playlist-header-actions">
      <button
        class="btn-download-all"
        class:downloading={downloadingPlaylists.has(selectedPlaylist.id)}
        onclick={() => selectedPlaylist && downloadPlaylist(selectedPlaylist)}
        disabled={downloadingPlaylists.has(selectedPlaylist.id) || loadingPlaylist || playlistTracks.length === 0}
      >
        {#if downloadingPlaylists.has(selectedPlaylist.id)}
          <span class="spinner"></span> {$_('search.playlist.adding')}
        {:else}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          {$_('search.playlist.downloadAll')} ({playlistTracks.length})
        {/if}
      </button>
    </div>
    {#if loadingPlaylist}
      <div class="status-message info"><span class="spinner"></span> {$_('search.playlist.loadingTracks')}</div>
    {:else if playlistError}
      <div class="status-message error">{playlistError}</div>
    {:else if playlistTracks.length > 0}
      <div class="results-header">
        <span class="col-title">{$_('search.track.title')}</span>
        <span class="col-album">{$_('search.track.album')}</span>
        <span class="col-duration">{$_('search.track.duration')}</span>
        <span class="col-action"></span>
      </div>
      <div class="results-list">
        {#each playlistTracks as track (track.id)}
          <div class="track-item">
            <button 
              class="btn-play-track"
              class:playing={isTrackPlaying(track)}
              onclick={() => playTrack(track)}
              disabled={!track.preview}
              title={track.preview ? (isTrackPlaying(track) ? 'Pause' : 'Play preview') : 'No preview available'}
            >
              {#if isTrackPlaying(track)}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="4" width="4" height="16" rx="1"/>
                  <rect x="14" y="4" width="4" height="16" rx="1"/>
                </svg>
              {:else}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z"/>
                </svg>
              {/if}
            </button>
            <img class="track-cover" src={track.cover_small} alt="" loading="lazy" />
            <div class="track-info">
              <div class="track-title">{track.title}</div>
              <button
                class="track-artist artist-link"
                onclick={() => openArtist(track.artist_id, track.artist, '')}
                title={$_('search.artist.browseDiscography', { values: { artist: track.artist } })}
              >{track.artist}</button>
            </div>
            <div class="track-album">{track.album}</div>
            <div class="track-duration">{formatDuration(track.duration)}</div>
            <div class="track-actions">
              <button 
                class="btn-lyrics"
                onclick={() => openLyrics(track)}
                title={$_('search.track.viewLyrics')}
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M9 18V5l12-2v13"/>
                  <circle cx="6" cy="18" r="3"/>
                  <circle cx="18" cy="16" r="3"/>
                </svg>
              </button>
              <button 
                class="btn-download {downloadStates.get(String(track.id)) === 'downloading' ? 'downloading' : ''} {downloadStates.get(String(track.id)) === 'complete' ? 'done' : ''}"
                onclick={() => downloadTrack(track)}
                title={$_('search.track.download')}
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
          </div>
        {/each}
      </div>
    {/if}

  <!-- Normal search results -->
  {:else}
    {#if searching}
      <div class="status-message info"><span class="spinner"></span> {$_('search.status.searching')}</div>
    {:else if errorMsg}
      <div class="status-message error">{errorMsg}</div>
    {/if}

    {#if searchType === 'tracks' && results.length > 0}
      <div class="results-header">
        <span class="col-title">{$_('search.track.title')}</span>
        <span class="col-album">{$_('search.track.album')}</span>
        <span class="col-duration">{$_('search.track.duration')}</span>
        <span class="col-action"></span>
      </div>
      <div class="results-list">
        {#each results as track (track.id)}
          <div class="track-item">
            <button 
              class="btn-play-track"
              class:playing={isTrackPlaying(track)}
              onclick={() => playTrack(track)}
              disabled={!track.preview}
              title={track.preview ? (isTrackPlaying(track) ? 'Pause' : 'Play preview') : 'No preview available'}
            >
              {#if isTrackPlaying(track)}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="4" width="4" height="16" rx="1"/>
                  <rect x="14" y="4" width="4" height="16" rx="1"/>
                </svg>
              {:else}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z"/>
                </svg>
              {/if}
            </button>
            <img class="track-cover" src={track.cover_small} alt="" loading="lazy" />
            <div class="track-info">
              <div class="track-title">{track.title}</div>
              <button
                class="track-artist artist-link"
                onclick={() => openArtist(track.artist_id, track.artist, '')}
                title={$_('search.artist.browseDiscography', { values: { artist: track.artist } })}
              >{track.artist}</button>
            </div>
            <div class="track-album">{track.album}</div>
            <div class="track-duration">{formatDuration(track.duration)}</div>
            <div class="track-actions">
              <button 
                class="btn-lyrics"
                onclick={() => openLyrics(track)}
                title={$_('search.track.viewLyrics')}
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M9 18V5l12-2v13"/>
                  <circle cx="6" cy="18" r="3"/>
                  <circle cx="18" cy="16" r="3"/>
                </svg>
              </button>
              <button 
                class="btn-download {downloadStates.get(String(track.id)) === 'downloading' ? 'downloading' : ''} {downloadStates.get(String(track.id)) === 'complete' ? 'done' : ''}"
                onclick={() => downloadTrack(track)}
                title={$_('search.track.download')}
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
                title={$_('search.artist.browseDiscography', { values: { artist: album.artist } })}
              >{album.artist}</button>
              {#if album.nb_tracks > 0}
                <div class="album-meta">{$_('search.album.tracks', { values: { count: album.nb_tracks } })}</div>
              {/if}
            </div>
            <button 
              class="btn-download-all"
              class:downloading={downloadingAlbums.has(album.id)}
              onclick={() => downloadAlbum(album)}
              disabled={downloadingAlbums.has(album.id)}
            >
              {#if downloadingAlbums.has(album.id)}
                <span class="spinner"></span> {$_('search.album.adding')}
              {:else}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                  <polyline points="7 10 12 15 17 10"/>
                  <line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                {$_('search.album.downloadAll')}
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
              {$_('search.artistCard.albums', { values: { count: artist.nb_album } })} · {$_('search.artistCard.fans', { values: { count: formatFans(artist.nb_fan) } })}
            </div>
          </button>
        {/each}
      </div>
    {/if}

    {#if searchType === 'playlists' && playlistResults.length > 0}
      <div class="results-list">
        {#each playlistResults as playlist (playlist.id)}
          <div class="album-item">
            <img class="album-cover" src={playlist.cover_medium} alt="" loading="lazy" />
            <div class="album-info">
              <div class="album-title">
                <button class="playlist-title-link" onclick={() => openPlaylist(playlist)}>{playlist.title}</button>
              </div>
              <div class="album-artist">{$_('search.playlist.by', { values: { creator: playlist.creator } })}</div>
              <div class="album-meta">{$_('search.playlist.tracks', { values: { count: playlist.nb_tracks } })}</div>
            </div>
            <button 
              class="btn-download-all"
              class:downloading={downloadingPlaylists.has(playlist.id)}
              onclick={() => downloadPlaylistFromResult(playlist)}
              disabled={downloadingPlaylists.has(playlist.id)}
            >
              {#if downloadingPlaylists.has(playlist.id)}
                <span class="spinner"></span> {$_('search.playlist.adding')}
              {:else}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                  <polyline points="7 10 12 15 17 10"/>
                  <line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                {$_('search.playlist.downloadAll')}
              {/if}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

{#if lyricsTrack}
  <LyricsModal track={lyricsTrack} onClose={closeLyrics} />
{/if}

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
  
  .search-bar-container {
    position: relative;
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

  .search-history-dropdown {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    right: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
    z-index: 100;
    max-height: 320px;
    overflow-y: auto;
  }

  .search-history-header {
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .search-history-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 16px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 14px;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
  }

  .search-history-item:hover {
    background: var(--bg-hover);
  }

  .search-history-item svg {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .search-history-item span {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    grid-template-columns: 40px 52px 1fr 160px 80px 132px;
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
    grid-column: 3;
  }
  
  .results-list {
    display: flex;
    flex-direction: column;
  }
  
  .track-item {
    display: grid;
    grid-template-columns: 40px 52px 1fr 160px 80px 132px;
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
  
  .btn-play-track {
    width: 32px;
    height: 32px;
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

  .btn-play-track:hover:not(:disabled) {
    background: var(--accent);
    color: white;
    transform: scale(1.1);
  }

  .btn-play-track.playing {
    background: var(--accent);
    color: white;
  }

  .btn-play-track:disabled {
    opacity: 0.3;
    cursor: not-allowed;
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

  .track-actions {
    display: flex;
    gap: 4px;
    align-items: center;
    justify-content: flex-end;
  }

  .btn-lyrics {
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

  .btn-lyrics:hover {
    background: var(--accent);
    color: white;
    transform: scale(1.1);
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

  .playlist-cover {
    border-radius: var(--radius-sm);
  }

  .playlist-title-link {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    font-family: inherit;
    font-size: inherit;
    font-weight: inherit;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    width: 100%;
    transition: color 0.15s;
  }

  .playlist-title-link:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  .album-artist {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .playlist-header-actions {
    display: flex;
    justify-content: flex-end;
    padding: 0 0 12px;
  }
</style>
