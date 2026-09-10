<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { loggedIn, downloads, searchHistory, audioPlayer, type Track } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import { searchRateLimiter } from '$lib/rateLimiter';
  import { onMount } from 'svelte';
  import { keyboardShortcuts } from '$lib/keyboardShortcuts';
  import { audioPlayerManager } from '$lib/audioPlayer';
  import { _ } from 'svelte-i18n';
  import SearchResults from './search/SearchResults.svelte';
  import type {
    AlbumResult,
    ArtistResult,
    PlaylistResult,
    SearchType,
    SelectedArtist,
    SelectedPlaylist
  } from './search/types';
  import './search/SearchView.css';

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

  // URL input state
  let urlInput = $state<string>('');
  let parsingUrl = $state<boolean>(false);
  let urlError = $state<string>('');

  // Audio player state
  let currentPlayingTrack = $state<Track | null>(null);
  let isPlaying = $state<boolean>(false);

  let searchTimeout: ReturnType<typeof setTimeout> | undefined;
  let activeSearchToken = 0;
  let activeArtistToken = 0;
  let activePlaylistToken = 0;

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
      activeSearchToken++;
      activeArtistToken++;
      activePlaylistToken++;
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
    } else if (searchQuery) {
      clearSearch();
    }
  }

  function clearSearch(): void {
    clearTimeout(searchTimeout);
    activeSearchToken += 1;
    searching = false;
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
    activeSearchToken += 1;
    searching = false;
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
    clearTimeout(searchTimeout);
    closeArtist();
    closePlaylist();
    activeSearchToken += 1;
    searching = false;
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
    clearTimeout(searchTimeout);
    if (!isLoggedIn) {
      errorMsg = $_('search.status.loginRequired');
      return;
    }

    const query = searchQuery.trim();
    if (!query) return;

    const requestToken = ++activeSearchToken;
    const requestedType = searchType;
    searching = true;
    errorMsg = '';
    resetResults();
    showSearchHistory = false;

    try {
      await searchRateLimiter.throttle();
      if (requestToken !== activeSearchToken || query !== searchQuery.trim() || requestedType !== searchType) {
        return;
      }

      const searchHandlers = {
        tracks: async () => {
          const data = await invoke<Track[]>('search_tracks', { query });
          if (requestToken !== activeSearchToken || query !== searchQuery.trim() || requestedType !== searchType) {
            return null;
          }
          results = data;
          return data.length;
        },
        albums: async () => {
          const data = await invoke<AlbumResult[]>('search_albums', { query });
          if (requestToken !== activeSearchToken || query !== searchQuery.trim() || requestedType !== searchType) {
            return null;
          }
          albumResults = data;
          return data.length;
        },
        artists: async () => {
          const data = await invoke<ArtistResult[]>('search_artists', { query });
          if (requestToken !== activeSearchToken || query !== searchQuery.trim() || requestedType !== searchType) {
            return null;
          }
          artistResults = data;
          return data.length;
        },
        playlists: async () => {
          const data = await invoke<PlaylistResult[]>('search_playlists', { query });
          if (requestToken !== activeSearchToken || query !== searchQuery.trim() || requestedType !== searchType) {
            return null;
          }
          playlistResults = data;
          return data.length;
        }
      };

      const resultCount = await searchHandlers[searchType]();
      if (resultCount === null) {
        return;
      }
      
      if (resultCount === 0) {
        errorMsg = $_('search.status.noResults');
      } else {
        await addToSearchHistory(query);
      }
    } catch (err) {
      if (requestToken !== activeSearchToken || query !== searchQuery.trim() || requestedType !== searchType) {
        return;
      }
      errorMsg = String(err);
    } finally {
      if (requestToken === activeSearchToken) {
        searching = false;
      }
    }
  }

  async function openArtist(id: number, name: string, picture: string): Promise<void> {
    const token = ++activeArtistToken;
    selectedArtist = { id, name, picture };
    artistAlbums = [];
    discographyError = '';
    loadingDiscography = true;

    try {
      const data = await invoke<AlbumResult[]>('get_artist_albums', { artistId: String(id) });
      if (token !== activeArtistToken) return;
      artistAlbums = data;
      if (artistAlbums.length === 0) {
        discographyError = $_('search.artist.noAlbums');
      }
    } catch (err) {
      if (token === activeArtistToken) discographyError = String(err);
    } finally {
      if (token === activeArtistToken) loadingDiscography = false;
    }
  }

  function closeArtist(): void {
    activeArtistToken++;
    loadingDiscography = false;
    selectedArtist = null;
    artistAlbums = [];
    discographyError = '';
  }

  async function openPlaylist(playlist: PlaylistResult): Promise<void> {
    const token = ++activePlaylistToken;
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
      if (token !== activePlaylistToken) return;
      playlistTracks = data;
      if (playlistTracks.length === 0) {
        playlistError = $_('search.playlist.noTracks');
      }
    } catch (err) {
      if (token === activePlaylistToken) playlistError = String(err);
    } finally {
      if (token === activePlaylistToken) loadingPlaylist = false;
    }
  }

  function closePlaylist(): void {
    activePlaylistToken++;
    loadingPlaylist = false;
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
    if (['resolving', 'downloading', 'tagging', 'complete'].includes(state ?? '')) return;
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

  async function handleUrlInput(): Promise<void> {
    if (!isLoggedIn) {
      urlError = $_('search.status.loginRequired');
      return;
    }

    const url = urlInput.trim();
    if (!url) {
      urlError = '';
      return;
    }

    parsingUrl = true;
    urlError = '';

    try {
      const parsed = await invoke<{ type: string; id: string }>('parse_deezer_url', { url });
      
      switch (parsed.type) {
        case 'track':
          const track = await invoke<Track>('get_track_by_id', { trackId: parsed.id });
          await downloadTrack(track);
          urlInput = '';
          break;
          
        case 'album':
          const albumTracks = await invoke<Track[]>('get_album_tracks', { albumId: parsed.id });
          for (const track of albumTracks) {
            await downloadQueueManager.addToQueue(track);
          }
          urlInput = '';
          break;
          
        case 'playlist':
          const playlistTracks = await invoke<Track[]>('get_playlist_tracks', { playlistId: parsed.id });
          for (const track of playlistTracks) {
            await downloadQueueManager.addToQueue(track);
          }
          urlInput = '';
          break;
          
        case 'artist':
          // For artists, we'll get their albums and download all tracks
          const artistAlbums = await invoke<AlbumResult[]>('get_artist_albums', { artistId: parsed.id });
          for (const album of artistAlbums) {
            const tracks = await invoke<Track[]>('get_album_tracks', { albumId: String(album.id) });
            for (const track of tracks) {
              await downloadQueueManager.addToQueue(track);
            }
          }
          urlInput = '';
          break;
          
        default:
          urlError = 'Unsupported content type';
      }
    } catch (err) {
      urlError = `Error: ${String(err)}`;
    } finally {
      parsingUrl = false;
    }
  }

  function playTrack(track: Track): void {
    audioPlayerManager.play(track);
  }

  function isTrackPlaying(track: Track): boolean {
    return currentPlayingTrack?.id === track.id && isPlaying;
  }

  function getDownloadButtonState(trackId: string): 'idle' | 'downloading' | 'complete' {
    const state = downloadStates.get(trackId);
    if (['resolving', 'downloading', 'tagging'].includes(state ?? '')) return 'downloading';
    if (state === 'complete') return 'complete';
    return 'idle';
  }
</script>

<div class="view search-view">
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
      
      <!-- URL Input Section -->
      <div class="url-input-container">
        <div class="url-input-header">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
          </svg>
          <span>Download from URL</span>
        </div>
        <div class="url-input-bar">
          <input 
            type="url" 
            bind:value={urlInput}
            onkeydown={(e) => e.key === 'Enter' && handleUrlInput()}
            placeholder="Paste Deezer track, album, playlist, or artist URL..."
            disabled={parsingUrl}
            class:url-error={urlError}
          />
          <button 
            class="url-download-btn" 
            onclick={handleUrlInput}
            disabled={parsingUrl || !urlInput.trim()}
          >
            {#if parsingUrl}
              <span class="spinner-small"></span>
            {:else}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            {/if}
          </button>
        </div>
        {#if urlError}
          <div class="url-error">{urlError}</div>
        {/if}
      </div>
    {/if}
  </div>

  <SearchResults
    {selectedArtist}
    {artistAlbums}
    {loadingDiscography}
    {discographyError}
    {downloadingAlbums}
    {downloadAlbum}
    {selectedPlaylist}
    {downloadingPlaylists}
    {downloadPlaylist}
    {loadingPlaylist}
    {playlistError}
    {playlistTracks}
    {isTrackPlaying}
    {playTrack}
    {openArtist}
    {downloadTrack}
    {downloadStates}
    {searching}
    {errorMsg}
    {searchType}
    {results}
    {albumResults}
    {artistResults}
    {playlistResults}
    {openPlaylist}
    {downloadPlaylistFromResult}
  />
</div>
