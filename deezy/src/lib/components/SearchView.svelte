<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { loggedIn, spotifyConnected, downloads, searchHistory, audioPlayer, type Track } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import { searchRateLimiter } from '$lib/rateLimiter';
  import { onMount } from 'svelte';
  import { keyboardShortcuts } from '$lib/keyboardShortcuts';
  import { audioPlayerManager } from '$lib/audioPlayer';
  import { _ } from 'svelte-i18n';
  import SearchResults from './search/SearchResults.svelte';
  import {
    isLibraryType,
    type AlbumResult,
    type ArtistResult,
    type CatalogId,
    type LibraryType,
    type PlaylistResult,
    type Provider,
    type SearchType,
    type SelectedArtist,
    type SelectedPlaylist
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
  let spotifyAvailable = $state<boolean>(false);
  let source = $state<Provider>('deezer');
  let downloadStates = $state<Map<string, string>>(new Map());
  let downloadingAlbums = $state<Set<CatalogId>>(new Set());
  let downloadingPlaylists = $state<Set<CatalogId>>(new Set());
  let downloadingLovedTracks = $state<boolean>(false);
  let showSearchHistory = $state<boolean>(false);
  let history = $state<string[]>([]);
  let searchInputRef = $state<HTMLInputElement | undefined>(undefined);

  // Artist discography state
  let selectedArtist = $state<SelectedArtist | null>(null);
  let artistAlbums = $state<AlbumResult[]>([]);
  let loadingDiscography = $state<boolean>(false);
  let discographyError = $state<string>('');

  // Album detail state
  let selectedAlbum = $state<AlbumResult | null>(null);
  let albumTracks = $state<Track[]>([]);
  let loadingAlbum = $state<boolean>(false);
  let albumError = $state<string>('');

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
  let activeAlbumToken = 0;
  let activePlaylistToken = 0;

  $effect(() => {
    try {
      const unsubscribe1 = loggedIn.subscribe(val => isLoggedIn = val);
      const unsubscribeSpotify = spotifyConnected.subscribe(val => spotifyAvailable = val);
      const unsubscribe2 = downloads.subscribe(val => downloadStates = val);
      const unsubscribe3 = searchHistory.subscribe(val => history = val);
      const unsubscribe4 = audioPlayer.subscribe(state => {
        currentPlayingTrack = state.currentTrack;
        isPlaying = state.isPlaying;
      });
      return () => {
        unsubscribe1();
        unsubscribeSpotify();
        unsubscribe2();
        unsubscribe3();
        unsubscribe4();
      };
    } catch (err) {
      console.error('Error subscribing to stores:', err);
    }
  });

  /** Whether the selected source can run a search right now. */
  function canSearch(): boolean {
    return source === 'spotify' ? spotifyAvailable : isLoggedIn;
  }

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
      activeAlbumToken++;
      activePlaylistToken++;
      document.removeEventListener('click', handleClickOutside);
      keyboardShortcuts.unregister('focus-search');
      keyboardShortcuts.unregister('clear-search');
      clearTimeout(searchTimeout);
    };
  });

  function handleEscapeAction(): void {
    if (selectedAlbum) {
      closeAlbum();
    } else if (selectedPlaylist) {
      closePlaylist();
    } else if (selectedArtist) {
      closeArtist();
    } else if (searchQuery && !isLibraryType(searchType)) {
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
    closeAlbum();
    closeArtist();
    closePlaylist();
    activeSearchToken += 1;
    searching = false;
    searchType = type;
    resetResults();
    errorMsg = '';
    selectedArtist = null;
    selectedPlaylist = null;
    if (isLibraryType(type)) {
      loadLibrary(type);
    } else if (searchQuery.trim().length >= 2) {
      doSearch();
    }
  }

  /** Switches between the Deezer and Spotify catalogs. */
  function switchSource(next: Provider): void {
    if (next === source) return;
    if (next === 'spotify' && !spotifyAvailable) {
      errorMsg = $_('search.source.spotifyRequired');
      return;
    }

    clearTimeout(searchTimeout);
    closeAlbum();
    closeArtist();
    closePlaylist();
    activeSearchToken += 1;
    searching = false;
    source = next;
    resetResults();
    errorMsg = '';
    showSearchHistory = false;

    if (isLibraryType(searchType)) {
      loadLibrary(searchType);
    } else if (searchQuery.trim().length >= 2) {
      doSearch();
    }
  }

  async function loadLibrary(type: LibraryType): Promise<void> {
    if (!canSearch()) {
      errorMsg = source === 'spotify'
        ? $_('search.source.spotifyRequired')
        : $_('search.status.loginRequired');
      return;
    }

    const requestToken = ++activeSearchToken;
    searching = true;
    errorMsg = '';
    resetResults();

    try {
      let count: number;
      if (type === 'loved') {
        const data = await invoke<Track[]>(
          source === 'spotify' ? 'get_spotify_loved_tracks' : 'get_loved_tracks'
        );
        if (requestToken !== activeSearchToken) return;
        results = data;
        count = data.length;
      } else {
        const data = await invoke<PlaylistResult[]>(
          source === 'spotify' ? 'get_spotify_user_playlists' : 'get_user_playlists'
        );
        if (requestToken !== activeSearchToken) return;
        playlistResults = data;
        count = data.length;
      }
      if (count === 0) {
        errorMsg = $_(type === 'loved' ? 'search.library.noLovedTracks' : 'search.library.noPlaylists');
      }
    } catch (err) {
      if (requestToken === activeSearchToken) errorMsg = String(err);
    } finally {
      if (requestToken === activeSearchToken) searching = false;
    }
  }

  async function downloadLovedTracks(): Promise<void> {
    if (downloadingLovedTracks) return;
    downloadingLovedTracks = true;
    try {
      for (const track of results) {
        await downloadQueueManager.addToQueue(track);
      }
    } finally {
      downloadingLovedTracks = false;
    }
  }
  
  async function doSearch(): Promise<void> {
    clearTimeout(searchTimeout);
    if (!canSearch()) {
      errorMsg = source === 'spotify'
        ? $_('search.source.spotifyRequired')
        : $_('search.status.loginRequired');
      return;
    }

    const query = searchQuery.trim();
    const requestedType = searchType;
    const requestedSource = source;
    if (!query || isLibraryType(requestedType)) return;

    const requestToken = ++activeSearchToken;
    const isStale = () =>
      requestToken !== activeSearchToken ||
      query !== searchQuery.trim() ||
      requestedType !== searchType ||
      requestedSource !== source;

    searching = true;
    errorMsg = '';
    resetResults();
    showSearchHistory = false;

    try {
      await searchRateLimiter.throttle();
      if (isStale()) {
        return;
      }

      const command = requestedSource === 'spotify'
        ? `search_spotify_${requestedType}`
        : `search_${requestedType}`;

      const searchHandlers = {
        tracks: async () => {
          const data = await invoke<Track[]>(command, { query });
          if (isStale()) return null;
          results = data;
          return data.length;
        },
        albums: async () => {
          const data = await invoke<AlbumResult[]>(command, { query });
          if (isStale()) return null;
          albumResults = data;
          return data.length;
        },
        artists: async () => {
          const data = await invoke<ArtistResult[]>(command, { query });
          if (isStale()) return null;
          artistResults = data;
          return data.length;
        },
        playlists: async () => {
          const data = await invoke<PlaylistResult[]>(command, { query });
          if (isStale()) return null;
          playlistResults = data;
          return data.length;
        }
      };

      const resultCount = await searchHandlers[requestedType]();
      if (resultCount === null) {
        return;
      }
      
      if (resultCount === 0) {
        errorMsg = $_('search.status.noResults');
      } else {
        await addToSearchHistory(query);
      }
    } catch (err) {
      if (isStale()) {
        return;
      }
      errorMsg = String(err);
    } finally {
      if (requestToken === activeSearchToken) {
        searching = false;
      }
    }
  }

  async function openArtist(id: CatalogId, name: string, picture: string): Promise<void> {
    closeAlbum();
    const token = ++activeArtistToken;
    selectedArtist = { id, name, picture };
    artistAlbums = [];
    discographyError = '';
    loadingDiscography = true;

    try {
      const command = source === 'spotify' ? 'get_spotify_artist_albums' : 'get_artist_albums';
      const data = await invoke<AlbumResult[]>(command, { artistId: String(id) });
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

  async function openAlbum(album: AlbumResult): Promise<void> {
    const token = ++activeAlbumToken;
    selectedAlbum = album;
    albumTracks = [];
    albumError = '';
    loadingAlbum = true;

    try {
      const command = album.provider === 'spotify' ? 'get_spotify_album_tracks' : 'get_album_tracks';
      const data = await invoke<Track[]>(command, { albumId: String(album.id) });
      if (token !== activeAlbumToken) return;
      albumTracks = data;
      if (albumTracks.length === 0) {
        albumError = $_('search.album.noTracks');
      }
    } catch (err) {
      if (token === activeAlbumToken) albumError = String(err);
    } finally {
      if (token === activeAlbumToken) loadingAlbum = false;
    }
  }

  function closeAlbum(): void {
    activeAlbumToken++;
    loadingAlbum = false;
    selectedAlbum = null;
    albumTracks = [];
    albumError = '';
  }

  async function openPlaylist(playlist: PlaylistResult): Promise<void> {
    const token = ++activePlaylistToken;
    selectedPlaylist = { 
      id: playlist.id, 
      title: playlist.title, 
      cover: playlist.cover_medium, 
      creator: playlist.creator,
      provider: playlist.provider
    };
    playlistTracks = [];
    playlistError = '';
    loadingPlaylist = true;

    try {
      const command = playlist.provider === 'spotify' ? 'get_spotify_playlist_tracks' : 'get_playlist_tracks';
      const data = await invoke<Track[]>(command, { playlistId: String(playlist.id) });
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
    const searchToken = activeSearchToken;

    try {
      let tracks = playlistTracks;
      if (tracks.length === 0) {
        const command = playlist.provider === 'spotify' ? 'get_spotify_playlist_tracks' : 'get_playlist_tracks';
        tracks = await invoke<Track[]>(command, { playlistId: String(playlist.id) });
      }
      for (const track of tracks) {
        await downloadQueueManager.addToQueue(track);
      }
    } catch (err) {
      // Don't show a stale error over a newer search.
      if (searchToken !== activeSearchToken) return;
      errorMsg = $_('search.playlist.downloadError', { values: { error: String(err) } });
    } finally {
      downloadingPlaylists = new Set([...downloadingPlaylists].filter(id => id !== playlist.id));
    }
  }

  async function downloadPlaylistFromResult(playlist: PlaylistResult): Promise<void> {
    if (downloadingPlaylists.has(playlist.id)) return;
    downloadingPlaylists = new Set([...downloadingPlaylists, playlist.id]);
    const searchToken = activeSearchToken;

    try {
      const command = playlist.provider === 'spotify' ? 'get_spotify_playlist_tracks' : 'get_playlist_tracks';
      const tracks = await invoke<Track[]>(command, { playlistId: String(playlist.id) });
      for (const track of tracks) {
        await downloadQueueManager.addToQueue(track);
      }
    } catch (err) {
      if (searchToken !== activeSearchToken) return;
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
    const searchToken = activeSearchToken;

    try {
      // Reuse the track list already loaded for the open album.
      const tracks = selectedAlbum?.id === album.id && albumTracks.length > 0
        ? albumTracks
        : await invoke<Track[]>(
            album.provider === 'spotify' ? 'get_spotify_album_tracks' : 'get_album_tracks',
            { albumId: String(album.id) }
          );
      for (const track of tracks) {
        await downloadQueueManager.addToQueue(track);
      }
    } catch (err) {
      if (searchToken !== activeSearchToken) return;
      errorMsg = $_('search.album.downloadError', { values: { error: String(err) } });
    } finally {
      downloadingAlbums = new Set([...downloadingAlbums].filter(id => id !== album.id));
    }
  }

  async function handleUrlInput(): Promise<void> {
    if (!isLoggedIn && !spotifyAvailable) {
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
      const parsed = await invoke<{ provider: Provider; type: string; id: string }>('parse_deezer_url', { url });
      const isSpotify = parsed.provider === 'spotify';
      
      switch (parsed.type) {
        case 'track':
          const track = await invoke<Track>(isSpotify ? 'get_spotify_track' : 'get_track_by_id', { trackId: parsed.id });
          await downloadTrack(track);
          urlInput = '';
          break;
          
        case 'album':
          const albumTracks = await invoke<Track[]>(
            isSpotify ? 'get_spotify_album_tracks' : 'get_album_tracks',
            { albumId: parsed.id }
          );
          for (const track of albumTracks) {
            await downloadQueueManager.addToQueue(track);
          }
          urlInput = '';
          break;
          
        case 'playlist':
          const playlistTracks = await invoke<Track[]>(
            isSpotify ? 'get_spotify_playlist_tracks' : 'get_playlist_tracks',
            { playlistId: parsed.id }
          );
          for (const track of playlistTracks) {
            await downloadQueueManager.addToQueue(track);
          }
          urlInput = '';
          break;
          
        case 'artist':
          // For artists, we'll get their albums and download all tracks
          const artistAlbums = await invoke<AlbumResult[]>(
            isSpotify ? 'get_spotify_artist_albums' : 'get_artist_albums',
            { artistId: parsed.id }
          );
          for (const album of artistAlbums) {
            const tracks = await invoke<Track[]>(
              isSpotify ? 'get_spotify_album_tracks' : 'get_album_tracks',
              { albumId: String(album.id) }
            );
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
    {#if selectedAlbum}
      <div class="artist-page-header">
        <button class="btn-back" onclick={closeAlbum}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          {selectedArtist ? $_('search.album.backToDiscography') : $_('search.album.back')}
        </button>
        <div class="artist-hero">
          {#if selectedAlbum.cover_medium}
            <img class="artist-hero-img playlist-cover" src={selectedAlbum.cover_medium} alt={selectedAlbum.title} />
          {:else}
            <div class="artist-hero-placeholder">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/>
              </svg>
            </div>
          {/if}
          <div class="artist-hero-info">
            <div class="artist-hero-name">{selectedAlbum.title}</div>
            <div class="artist-hero-meta">{selectedAlbum.artist || selectedArtist?.name || ''}</div>
          </div>
        </div>
      </div>
    {:else if selectedArtist}
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
      <div class="source-switch" role="group" aria-label={$_('search.source.label')}>
        <button
          class="source-btn"
          class:active={source === 'deezer'}
          onclick={() => switchSource('deezer')}
        >{$_('search.source.deezer')}</button>
        <button
          class="source-btn"
          class:active={source === 'spotify'}
          onclick={() => switchSource('spotify')}
        >{$_('search.source.spotify')}</button>
      </div>
      <div class="search-tabs">
        <button class="tab-btn" class:active={searchType === 'tracks'} onclick={() => switchSearchType('tracks')}>{$_('search.tabs.tracks')}</button>
        <button class="tab-btn" class:active={searchType === 'albums'} onclick={() => switchSearchType('albums')}>{$_('search.tabs.albums')}</button>
        <button class="tab-btn" class:active={searchType === 'artists'} onclick={() => switchSearchType('artists')}>{$_('search.tabs.artists')}</button>
        <button class="tab-btn" class:active={searchType === 'playlists'} onclick={() => switchSearchType('playlists')}>{$_('search.tabs.playlists')}</button>
        <span class="tab-divider" aria-hidden="true"></span>
        <button class="tab-btn" class:active={searchType === 'loved'} onclick={() => switchSearchType('loved')}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M12 21s-7.5-4.6-9.6-9.3C.9 8.3 3 4.5 6.7 4.5c2.1 0 3.6 1.2 5.3 3.1 1.7-1.9 3.2-3.1 5.3-3.1 3.7 0 5.8 3.8 4.3 7.2C19.5 16.4 12 21 12 21z"/>
          </svg>
          {$_('search.tabs.loved')}
        </button>
        <button class="tab-btn" class:active={searchType === 'myPlaylists'} onclick={() => switchSearchType('myPlaylists')}>{$_('search.tabs.myPlaylists')}</button>
      </div>
      {#if !isLibraryType(searchType)}
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
            placeholder="Paste a Deezer or Spotify track, album, playlist, or artist URL..."
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
    {selectedAlbum}
    {albumTracks}
    {loadingAlbum}
    {albumError}
    {openAlbum}
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
    {downloadingLovedTracks}
    {downloadLovedTracks}
  />
</div>
