<script lang="ts">
  import { downloadQueue, type QueuedDownload } from '$lib/stores';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import { dndzone, type DndEvent } from 'svelte-dnd-action';
  import { _ } from 'svelte-i18n';

  type QueueItemWithId = QueuedDownload & { id: string };

  let queueItems = $state<QueueItemWithId[]>([]);
  let dragDisabled = $state(true);

  $effect(() => {
    try {
      const unsubscribe = downloadQueue.subscribe(val => {
        queueItems = val.map((item) => ({
          ...item,
          id: String(item.track.id)
        }));
      });
      return unsubscribe;
    } catch (err) {
      console.error('Error subscribing to download queue:', err);
    }
  });

  function handleDndConsider(e: CustomEvent<DndEvent<QueueItemWithId>>): void {
    queueItems = e.detail.items;
  }

  function handleDndFinalize(e: CustomEvent<DndEvent<QueueItemWithId>>): void {
    queueItems = e.detail.items;
    dragDisabled = true;
    
    const reorderedQueue = queueItems.map(item => ({
      track: item.track,
      priority: item.priority
    }));
    
    downloadQueueManager.reorderQueue(reorderedQueue);
  }

  function removeFromQueue(trackId: string): void {
    downloadQueueManager.removeFromQueue(trackId);
  }

  function startDrag(): void {
    dragDisabled = false;
  }

  function endDrag(): void {
    dragDisabled = true;
  }

  function getTrackSubtitle(item: QueueItemWithId): string {
    const parts = [];
    if (item.track.artist) parts.push(item.track.artist);
    if (item.track.album) parts.push(item.track.album);
    return parts.join(' • ');
  }
</script>

{#if queueItems.length > 0}
  <section class="queue-section" aria-labelledby="queue-title">
    <h3 id="queue-title">{$_('downloads.queue.title')} ({queueItems.length})</h3>
    <div 
      class="queue-list"
      use:dndzone={{items: queueItems, dragDisabled, dropTargetStyle: {}}}
      onconsider={handleDndConsider}
      onfinalize={handleDndFinalize}
      role="list"
      aria-label="Download queue"
    >
      {#each queueItems as item (item.id)}
        <div class="queue-item" data-id={item.id} role="listitem">
          <button 
            class="drag-handle" 
            aria-label="Drag to reorder"
            title="Drag to reorder"
            onmousedown={startDrag}
            onmouseup={endDrag}
            ontouchstart={startDrag}
            ontouchend={endDrag}
            type="button"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
              <line x1="3" y1="6" x2="21" y2="6"/>
              <line x1="3" y1="12" x2="21" y2="12"/>
              <line x1="3" y1="18" x2="21" y2="18"/>
            </svg>
          </button>
          
          {#if item.track.cover_medium || item.track.cover_small}
            <img 
              class="queue-cover" 
              src={item.track.cover_medium || item.track.cover_small} 
              alt={item.track.title}
              loading="lazy"
            />
          {:else}
            <div class="queue-cover" role="img" aria-label="No cover"></div>
          {/if}
          
          <div class="queue-details">
            <div class="queue-title" title={item.track.title}>{item.track.title}</div>
            <div class="queue-sub">
              {#if item.track.artist}
                <span>{item.track.artist}</span>
              {/if}
              {#if item.track.artist && item.track.album}
                <span class="separator" aria-hidden="true">•</span>
              {/if}
              {#if item.track.album}
                <span>{item.track.album}</span>
              {/if}
            </div>
          </div>
          
          <button 
            class="remove-btn" 
            title={$_('downloads.queue.remove')}
            aria-label="{$_('downloads.queue.remove')} {item.track.title}"
            onclick={() => removeFromQueue(String(item.track.id))}
            type="button"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      {/each}
    </div>
  </section>
{/if}

<style>
  .queue-section {
    margin-bottom: 32px;
  }

  h3 {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--text-secondary);
  }

  .queue-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .queue-item {
    display: grid;
    grid-template-columns: 32px 44px 1fr auto;
    gap: 12px;
    align-items: center;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    transition: all 0.15s ease;
  }

  .queue-item:hover {
    background: var(--bg-elevated);
  }

  .drag-handle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    cursor: grab;
    border-radius: var(--radius-sm);
    transition: all 0.15s ease;
    padding: 0;
  }

  .drag-handle:hover {
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .queue-cover {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-elevated);
  }

  .queue-details {
    overflow: hidden;
  }

  .queue-title {
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .queue-sub {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .queue-sub .separator {
    opacity: 0.5;
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    transition: all 0.15s ease;
    padding: 0;
  }

  .remove-btn:hover {
    background: var(--bg-elevated);
    color: var(--error);
    border-color: var(--error);
  }
</style>
