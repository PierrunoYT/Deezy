<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { formatDuration, formatFans } from '$lib/i18n/formatters';
  import type { Track } from '$lib/stores';
  import { isLibraryType, type AlbumResult, type ArtistResult, type CatalogId, type PlaylistResult, type SearchType, type SelectedArtist, type SelectedPlaylist } from './types';

  interface Props {
    selectedArtist: SelectedArtist | null;
    artistAlbums: AlbumResult[];
    loadingDiscography: boolean;
    discographyError: string;
    downloadingAlbums: Set<CatalogId>;
    downloadAlbum: (album: AlbumResult) => Promise<void>;
    selectedAlbum: AlbumResult | null;
    albumTracks: Track[];
    loadingAlbum: boolean;
    albumError: string;
    openAlbum: (album: AlbumResult) => Promise<void>;
    selectedPlaylist: SelectedPlaylist | null;
    downloadingPlaylists: Set<CatalogId>;
    downloadPlaylist: (playlist: SelectedPlaylist) => Promise<void>;
    loadingPlaylist: boolean;
    playlistError: string;
    playlistTracks: Track[];
    isTrackPlaying: (track: Track) => boolean;
    playTrack: (track: Track) => void;
    openArtist: (id: CatalogId, name: string, picture: string) => Promise<void>;
    downloadTrack: (track: Track) => Promise<void>;
    downloadStates: Map<string, string>;
    searching: boolean;
    errorMsg: string;
    searchType: SearchType;
    results: Track[];
    albumResults: AlbumResult[];
    artistResults: ArtistResult[];
    playlistResults: PlaylistResult[];
    openPlaylist: (playlist: PlaylistResult) => Promise<void>;
    downloadPlaylistFromResult: (playlist: PlaylistResult) => Promise<void>;
    downloadingLovedTracks: boolean;
    downloadLovedTracks: () => Promise<void>;
  }

  let { selectedArtist, artistAlbums, loadingDiscography, discographyError, downloadingAlbums,
    downloadAlbum, selectedAlbum, albumTracks, loadingAlbum, albumError, openAlbum,
    selectedPlaylist, downloadingPlaylists, downloadPlaylist, loadingPlaylist,
    playlistError, playlistTracks, isTrackPlaying, playTrack, openArtist, downloadTrack,
    downloadStates, searching, errorMsg, searchType, results, albumResults, artistResults,
    playlistResults, openPlaylist, downloadPlaylistFromResult, downloadingLovedTracks,
    downloadLovedTracks }: Props = $props();
</script>

{#snippet downloadIcon()}
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
    <polyline points="7 10 12 15 17 10"/>
    <line x1="12" y1="15" x2="12" y2="3"/>
  </svg>
{/snippet}

{#snippet trackList(tracks: Track[])}
  <div class="results-header">
    <span class="col-title">{$_('search.track.title')}</span>
    <span class="col-album">{$_('search.track.album')}</span>
    <span class="col-duration">{$_('search.track.duration')}</span>
    <span class="col-action"></span>
  </div>
  <div class="results-list">
    {#each tracks as track (track.id)}
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
{/snippet}

{#snippet albumDownloadButton(album: AlbumResult)}
  <button
    class="btn-download-all"
    class:downloading={downloadingAlbums.has(album.id)}
    onclick={() => downloadAlbum(album)}
    disabled={downloadingAlbums.has(album.id)}
  >
    {#if downloadingAlbums.has(album.id)}
      <span class="spinner"></span> {$_('search.album.adding')}
    {:else}
      {@render downloadIcon()}
      {$_('search.album.downloadAll')}
    {/if}
  </button>
{/snippet}

<!-- Album detail view (from album search results or an artist's discography) -->
  {#if selectedAlbum}
    <div class="playlist-header-actions">
      <button
        class="btn-download-all"
        class:downloading={downloadingAlbums.has(selectedAlbum.id)}
        onclick={() => selectedAlbum && downloadAlbum(selectedAlbum)}
        disabled={downloadingAlbums.has(selectedAlbum.id) || loadingAlbum || albumTracks.length === 0}
      >
        {#if downloadingAlbums.has(selectedAlbum.id)}
          <span class="spinner"></span> {$_('search.album.adding')}
        {:else}
          {@render downloadIcon()}
          {$_('search.album.downloadAll')} ({albumTracks.length})
        {/if}
      </button>
    </div>
    {#if loadingAlbum}
      <div class="status-message info"><span class="spinner"></span> {$_('search.album.loadingTracks')}</div>
    {:else if albumError}
      <div class="status-message error">{albumError}</div>
    {:else if albumTracks.length > 0}
      {@render trackList(albumTracks)}
    {/if}

<!-- Artist discography view -->
  {:else if selectedArtist}
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
              <div class="album-title">
                <button
                  class="playlist-title-link"
                  onclick={() => openAlbum(album)}
                  title={$_('search.album.viewTracks')}
                >{album.title}</button>
              </div>
              {#if album.nb_tracks > 0}
                <div class="album-meta">{$_('search.album.tracks', { values: { count: album.nb_tracks } })}</div>
              {/if}
            </div>
            {@render albumDownloadButton(album)}
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
          {@render downloadIcon()}
          {$_('search.playlist.downloadAll')} ({playlistTracks.length})
        {/if}
      </button>
    </div>
    {#if loadingPlaylist}
      <div class="status-message info"><span class="spinner"></span> {$_('search.playlist.loadingTracks')}</div>
    {:else if playlistError}
      <div class="status-message error">{playlistError}</div>
    {:else if playlistTracks.length > 0}
      {@render trackList(playlistTracks)}
    {/if}

  <!-- Normal search results -->
  {:else}
    {#if searching}
      <div class="status-message info">
        <span class="spinner"></span>
        {isLibraryType(searchType) ? $_('search.library.loading') : $_('search.status.searching')}
      </div>
    {:else if errorMsg}
      <div class="status-message error">{errorMsg}</div>
    {/if}

    {#if searchType === 'tracks' && results.length > 0}
      {@render trackList(results)}
    {/if}

    {#if searchType === 'loved' && !searching && results.length > 0}
      <div class="playlist-header-actions">
        <button
          class="btn-download-all"
          class:downloading={downloadingLovedTracks}
          onclick={downloadLovedTracks}
          disabled={downloadingLovedTracks}
        >
          {#if downloadingLovedTracks}
            <span class="spinner"></span> {$_('search.playlist.adding')}
          {:else}
            {@render downloadIcon()}
            {$_('search.playlist.downloadAll')} ({results.length})
          {/if}
        </button>
      </div>
      {@render trackList(results)}
    {/if}

    {#if searchType === 'albums' && albumResults.length > 0}
      <div class="results-list">
        {#each albumResults as album (album.id)}
          <div class="album-item">
            <img class="album-cover" src={album.cover_medium} alt="" loading="lazy" />
            <div class="album-info">
              <div class="album-title">
                <button
                  class="playlist-title-link"
                  onclick={() => openAlbum(album)}
                  title={$_('search.album.viewTracks')}
                >{album.title}</button>
              </div>
              <button
                class="album-artist artist-link"
                onclick={() => openArtist(album.artist_id, album.artist, '')}
                title={$_('search.artist.browseDiscography', { values: { artist: album.artist } })}
              >{album.artist}</button>
              {#if album.nb_tracks > 0}
                <div class="album-meta">{$_('search.album.tracks', { values: { count: album.nb_tracks } })}</div>
              {/if}
            </div>
            {@render albumDownloadButton(album)}
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

    {#if (searchType === 'playlists' || searchType === 'myPlaylists') && playlistResults.length > 0}
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
                {@render downloadIcon()}
                {$_('search.playlist.downloadAll')}
              {/if}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
