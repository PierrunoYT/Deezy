<script lang="ts">
  import { audioPlayer } from '$lib/stores';
  import { audioPlayerManager } from '$lib/audioPlayer';
  import { onMount } from 'svelte';

  let playerState = $state($audioPlayer);
  let isDraggingSeek = $state(false);
  let isDraggingVolume = $state(false);
  let seekBarRef = $state<HTMLDivElement | undefined>(undefined);
  let volumeBarRef = $state<HTMLDivElement | undefined>(undefined);

  $effect(() => {
    const unsubscribe = audioPlayer.subscribe(state => {
      playerState = state;
    });
    return unsubscribe;
  });

  function togglePlayPause() {
    audioPlayerManager.togglePlayPause();
  }

  function handleSeekMouseDown(e: MouseEvent) {
    isDraggingSeek = true;
    updateSeek(e);
  }

  function handleSeekMouseMove(e: MouseEvent) {
    if (isDraggingSeek) {
      updateSeek(e);
    }
  }

  function handleSeekMouseUp() {
    isDraggingSeek = false;
  }

  function updateSeek(e: MouseEvent) {
    if (!seekBarRef) return;
    const rect = seekBarRef.getBoundingClientRect();
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const time = percent * playerState.duration;
    audioPlayerManager.seek(time);
  }

  function handleVolumeMouseDown(e: MouseEvent) {
    isDraggingVolume = true;
    updateVolume(e);
  }

  function handleVolumeMouseMove(e: MouseEvent) {
    if (isDraggingVolume) {
      updateVolume(e);
    }
  }

  function handleVolumeMouseUp() {
    isDraggingVolume = false;
  }

  function updateVolume(e: MouseEvent) {
    if (!volumeBarRef) return;
    const rect = volumeBarRef.getBoundingClientRect();
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    audioPlayerManager.setVolume(percent);
  }

  function formatTime(seconds: number): string {
    if (!isFinite(seconds)) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  onMount(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isDraggingSeek) handleSeekMouseMove(e);
      if (isDraggingVolume) handleVolumeMouseMove(e);
    };

    const handleMouseUp = () => {
      handleSeekMouseUp();
      handleVolumeMouseUp();
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
  <div class="mini-player">
    <div class="player-track-info">
      <img 
        class="player-cover" 
        src={playerState.currentTrack.cover_small} 
        alt={playerState.currentTrack.title}
      />
      <div class="player-text">
        <div class="player-title">{playerState.currentTrack.title}</div>
        <div class="player-artist">{playerState.currentTrack.artist}</div>
      </div>
    </div>

    <div class="player-controls">
      <button 
        class="btn-play-pause" 
        onclick={togglePlayPause}
        title={playerState.isPlaying ? 'Pause' : 'Play'}
      >
        {#if playerState.isPlaying}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="4" width="4" height="16" rx="1"/>
            <rect x="14" y="4" width="4" height="16" rx="1"/>
          </svg>
        {:else}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z"/>
          </svg>
        {/if}
      </button>

      <div class="player-time">
        {formatTime(playerState.currentTime)}
      </div>

      <div 
        class="seek-bar" 
        bind:this={seekBarRef}
        onmousedown={handleSeekMouseDown}
      >
        <div class="seek-bar-bg">
          <div 
            class="seek-bar-progress" 
            style="width: {playerState.duration > 0 ? (playerState.currentTime / playerState.duration) * 100 : 0}%"
          ></div>
        </div>
      </div>

      <div class="player-time">
        {formatTime(playerState.duration)}
      </div>
    </div>

    <div class="player-volume">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
      </svg>
      <div 
        class="volume-bar" 
        bind:this={volumeBarRef}
        onmousedown={handleVolumeMouseDown}
      >
        <div class="volume-bar-bg">
          <div 
            class="volume-bar-progress" 
            style="width: {playerState.volume * 100}%"
          ></div>
        </div>
      </div>
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
