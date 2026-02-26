<script lang="ts">
  import { keyboardShortcuts, KeyboardShortcutsManager } from '$lib/keyboardShortcuts';
  import { onMount } from 'svelte';
  
  interface Props {
    show: boolean;
    onClose: () => void;
  }
  
  let { show, onClose }: Props = $props();
  
  let shortcuts = $state<Record<string, any[]>>({
    navigation: [],
    search: [],
    general: []
  });

  let modalRef = $state<HTMLDivElement | undefined>(undefined);
  
  $effect(() => {
    if (show) {
      shortcuts = keyboardShortcuts.getAllShortcuts();
      setTimeout(() => modalRef?.focus(), 0);
    }
  });
  
  function handleBackdropClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }
  
  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }
  
  const categoryTitles: Record<string, string> = {
    navigation: 'Navigation',
    search: 'Search & Actions',
    general: 'General'
  };

  const categoryOrder = ['navigation', 'search', 'general'];

  function getOrderedCategories(): [string, any[]][] {
    return categoryOrder
      .map(cat => [cat, shortcuts[cat]] as [string, any[]])
      .filter(([_, items]) => items.length > 0);
  }
</script>

{#if show}
  <div 
    class="modal-backdrop" 
    onclick={handleBackdropClick} 
    onkeydown={handleKeydown} 
    role="dialog" 
    aria-modal="true" 
    aria-labelledby="shortcuts-title"
    bind:this={modalRef}
    tabindex="-1"
  >
    <div class="modal">
      <div class="modal-header">
        <h2 id="shortcuts-title">Keyboard Shortcuts</h2>
        <button class="close-btn" onclick={onClose} aria-label="Close dialog">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
      
      <div class="modal-content">
        {#each getOrderedCategories() as [category, items] (category)}
          <section class="shortcut-category" aria-labelledby="category-{category}">
            <h3 id="category-{category}">{categoryTitles[category]}</h3>
            <div class="shortcut-list" role="list">
              {#each items as shortcut (shortcut.key + shortcut.description)}
                <div class="shortcut-item" role="listitem">
                  <span class="shortcut-description">{shortcut.description}</span>
                  <kbd class="shortcut-keys" aria-label="Keyboard shortcut: {KeyboardShortcutsManager.formatShortcut(shortcut)}">
                    {KeyboardShortcutsManager.formatShortcut(shortcut)}
                  </kbd>
                </div>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.15s ease;
  }
  
  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  
  .modal {
    background: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 90%;
    max-width: 600px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s ease;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }
  
  @keyframes slideUp {
    from {
      transform: translateY(20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }
  
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
  }
  
  .modal-header h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0;
  }
  
  .close-btn {
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
  
  .close-btn:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  
  .modal-content {
    padding: 24px;
    overflow-y: auto;
    flex: 1;
  }
  
  .shortcut-category {
    margin-bottom: 28px;
  }
  
  .shortcut-category:last-child {
    margin-bottom: 0;
  }
  
  .shortcut-category h3 {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-tertiary);
    margin-bottom: 12px;
  }
  
  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  
  .shortcut-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--bg-elevated);
    border-radius: var(--radius);
    transition: background 0.15s;
  }
  
  .shortcut-item:hover {
    background: var(--bg-hover);
  }
  
  .shortcut-description {
    font-size: 14px;
    color: var(--text-primary);
  }
  
  .shortcut-keys {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--bg-dark);
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo, Consolas, 'DejaVu Sans Mono', monospace;
    white-space: nowrap;
  }
</style>
