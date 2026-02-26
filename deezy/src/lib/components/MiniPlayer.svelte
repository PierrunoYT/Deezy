<script lang="ts">
  import { audioPlayer } from '$lib/stores';
  import { audioPlayerManager } from '$lib/audioPlayer';
  import { onMount, onDestroy } from 'svelte';

  let playerState = $state($audioPlayer);
  let isDraggingSeek = $state(false);
  let isDraggingVolume = $state(false);
  let seekBarRef = $state<HTMLButtonElement | undefined>(undefined);
  let volumeBarRef = $state<HTMLButtonElement | undefined>(undefined);

  $effect(() => {
    const unsubscribe = audioPlayer.subscribe(state => {
      playerState = state;
    });
    return unsubscribe;
  });

  function togglePlayPause(): void {
    audioPlayerManager.togglePlayPause();
  }

  function clampPercent(value: number): number {
    return Math.max(0, Math.min(1, value));
  }

  function updateSeek(e: MouseEvent): void {
    if (!seekBarRef) return;
    const rect = seekBarRef.getBoundingClientRect();
    const percent = clampPercent((e.clientX - rect.left) / rect.width);
    const time = percent * playerState.duration;
    audioPlayerManager.seek(time);
  }

  function updateVolume(e: MouseEvent): void {
    if (!volumeBarRef) return;
    const rect = volumeBarRef.getBoundingClientRect();
    const percent = clampPercent((e.clientX - rect.left) / rect.width);
    audioPlayerManager.setVolume(percent);
  }

  function handleSeekMouseDown(e: MouseEvent): void {
    isDraggingSeek = true;
    updateSeek(e);
  }

  function handleVolumeMouseDown(e: MouseEvent): void {
    isDraggingVolume = true;
    updateVolume(e);
  }

  function formatTime(seconds: number): string {
    if (!isFinite(seconds)) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function getSeekPercentage(): number {
    return playerState.duration > 0 ? (playerState.currentTime / playerState.duration) * 100 : 0;
  }

  function getVolumePercentage(): number {
    return playerState.volume * 100;
  }

  onMount(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isDraggingSeek) updateSeek(e);
      if (isDraggingVolume) updateVolume(e);
    };

    const handleMouseUp = () => {
      isDraggingSeek = false;
      isDraggingVolume = false;
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  });
</script>

{#if playerState.currentTrack}
  <div class="mini-player" role="region" aria-label="Audio player">
    <div class="player-track-info">
      <img 
        class="player-cover" 
        src={playerState.currentTrack.cover_small} 
        alt={playerState.currentTrack.title}
        loading="lazy"
      />
      <div class="player-text">
        <div class="player-title" title={playerState.currentTrack.title}>{playerState.currentTrack.title}</div>
        <div class="player-artist" title={playerState.currentTrack.artist}>{playerState.currentTrack.artist}</div>
      </div>
    </div>

    <div class="player-controls">
      <button 
        class="btn-play-pause" 
        onclick={togglePlayPause}
        aria-label={playerState.isPlaying ? 'Pause' : 'Play'}
        title={playerState.isPlaying ? 'Pause' : 'Play'}
      >
        {#if playerState.isPlaying}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <rect x="6" y="4" width="4" height="16" rx="1"/>
            <rect x="14" y="4" width="4" height="16" rx="1"/>
          </svg>
        {:else}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M8 5v14l11-7z"/>
          </svg>
        {/if}
      </button>

      <time class="player-time" datetime="PT{Math.floor(playerState.currentTime)}S">
        {formatTime(playerState.currentTime)}
      </time>

      <button
        class="seek-bar" 
        type="button"
        bind:this={seekBarRef}
        onmousedown={handleSeekMouseDown}
        aria-label="Seek playback position"
        aria-valuemin={0}
        aria-valuemax={playerState.duration}
        aria-valuenow={playerState.currentTime}
        role="slider"
      >
        <div class="seek-bar-bg">
          <div 
            class="seek-bar-progress" 
            style="width: {getSeekPercentage()}%"
          ></div>
        </div>
      </button>

      <time class="player-time" datetime="PT{Math.floor(playerState.duration)}S">
        {formatTime(playerState.duration)}
      </time>
    </div>

    <div class="player-volume">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
      </svg>
      <button
        class="volume-bar" 
        type="button"
        bind:this={volumeBarRef}
        onmousedown={handleVolumeMouseDown}
        aria-label="Adjust volume"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(playerState.volume * 100)}
        role="slider"
      >
        <div class="volume-bar-bg">
          <div 
            class="volume-bar-progress" 
            style="width: {getVolumePercentage()}%"
          ></div>
        </div>
      </button>
    </div>
  </div>
{/if}

<style>
  .mini-player {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 80px;
    background: var(--bg-elevated);
    border-top: 1px solid var(--border);
    display: grid;
    grid-template-columns: 280px 1fr 200px;
    gap: 20px;
    align-items: center;
    padding: 0 24px;
    z-index: 100;
  }

  .player-track-info {
    display: flex;
    align-items: center;
    gap: 12px;
    overflow: hidden;
  }

  .player-cover {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-dark);
    flex-shrink: 0;
  }

  .player-text {
    overflow: hidden;
    flex: 1;
  }

  .player-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .player-artist {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .player-controls {
    display: flex;
    align-items: center;
    gap: 12px;
    justify-content: center;
  }

  .btn-play-pause {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: var(--accent);
    color: white;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    flex-shrink: 0;
  }

  .btn-play-pause:hover {
    transform: scale(1.05);
    box-shadow: 0 4px 12px rgba(162, 56, 255, 0.3);
  }

  .btn-play-pause:active {
    transform: scale(0.95);
  }

  .player-time {
    font-size: 12px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    min-width: 40px;
    text-align: center;
  }

  .seek-bar {
    flex: 1;
    cursor: pointer;
    padding: 8px 0;
    border: none;
    background: transparent;
  }

  .seek-bar-bg {
    height: 4px;
    background: var(--bg-dark);
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }

  .seek-bar-progress {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.1s linear;
  }

  .seek-bar:hover .seek-bar-bg {
    height: 6px;
  }

  .player-volume {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .player-volume svg {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .volume-bar {
    flex: 1;
    cursor: pointer;
    padding: 8px 0;
    border: none;
    background: transparent;
  }

  .volume-bar-bg {
    height: 4px;
    background: var(--bg-dark);
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }

  .volume-bar-progress {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
  }

  .volume-bar:hover .volume-bar-bg {
    height: 6px;
  }

  @media (max-width: 1024px) {
    .mini-player {
      grid-template-columns: 1fr;
      height: auto;
      padding: 16px;
      gap: 12px;
    }

    .player-volume {
      display: none;
    }
  }
</style>
