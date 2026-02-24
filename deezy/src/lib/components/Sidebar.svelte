<script lang="ts">
  import type { UserInfo } from '$lib/stores';
  import { _ } from 'svelte-i18n';

  interface Props {
    currentView: string;
    user: UserInfo | null;
    activeDownloads: number;
    onViewChange: (view: string) => void;
    onShowHelp?: () => void;
  }

  let { currentView, user, activeDownloads, onViewChange, onShowHelp }: Props = $props();
</script>

<nav id="sidebar">
  <div class="logo">
    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M9 18V5l12-2v13"/>
      <circle cx="6" cy="18" r="3"/>
      <circle cx="18" cy="16" r="3"/>
    </svg>
    <span>{$_('app.name')}</span>
  </div>
  
  <div class="nav-items">
    <button 
      class="nav-btn {currentView === 'search' ? 'active' : ''}" 
      onclick={() => onViewChange('search')}
    >
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      {$_('nav.search')}
    </button>
    
    <button 
      class="nav-btn {currentView === 'downloads' ? 'active' : ''}" 
      onclick={() => onViewChange('downloads')}
    >
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
        <polyline points="7 10 12 15 17 10"/>
        <line x1="12" y1="15" x2="12" y2="3"/>
      </svg>
      {$_('nav.downloads')}
      {#if activeDownloads > 0}
        <span class="badge">{activeDownloads}</span>
      {/if}
    </button>
    
    <button 
      class="nav-btn {currentView === 'settings' ? 'active' : ''}" 
      onclick={() => onViewChange('settings')}
    >
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
      {$_('nav.settings')}
    </button>
  </div>
  
  {#if onShowHelp}
    <button class="help-btn" onclick={onShowHelp} title="{$_('nav.keyboardShortcuts')} (Shift+?)">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
        <line x1="12" y1="17" x2="12.01" y2="17"/>
      </svg>
      <span>{$_('nav.keyboardShortcuts')}</span>
    </button>
  {/if}
  
  {#if user}
    <div class="user-info">
      {#if user.image}
        <img
          src={user.image}
          alt={user.name}
          onerror={(e) => {
            const img = e.currentTarget as HTMLImageElement;
            img.style.display = 'none';
            const fallback = img.nextElementSibling as HTMLElement | null;
            if (fallback) fallback.style.display = 'flex';
          }}
        />
        <span class="avatar-fallback" aria-hidden="true">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
            <circle cx="12" cy="7" r="4"/>
          </svg>
        </span>
      {:else}
        <span class="avatar-fallback avatar-fallback--visible" aria-hidden="true">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
            <circle cx="12" cy="7" r="4"/>
          </svg>
        </span>
      {/if}
      <span>{user.name || $_('user.connected')}</span>
    </div>
  {/if}
</nav>

<style>
  #sidebar {
    width: var(--sidebar-width);
    background: var(--bg-darkest);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 20px 12px;
    flex-shrink: 0;
  }
  
  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px 24px;
    font-size: 20px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -0.5px;
  }
  
  .logo svg {
    color: var(--accent);
  }
  
  .nav-items {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  
  .nav-btn {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    border-radius: var(--radius);
    cursor: pointer;
    transition: all 0.15s ease;
    position: relative;
    font-family: inherit;
  }
  
  .nav-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .nav-btn.active {
    background: var(--accent-dim);
    color: var(--accent);
  }
  
  .nav-btn.active svg {
    color: var(--accent);
  }
  
  .badge {
    background: var(--accent);
    color: white;
    font-size: 11px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 10px;
    margin-left: auto;
  }
  
  .help-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    font-size: 13px;
    font-weight: 500;
    border-radius: var(--radius);
    cursor: pointer;
    transition: all 0.15s ease;
    font-family: inherit;
    margin-bottom: 8px;
  }
  
  .help-btn:hover {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }
  
  .help-btn svg {
    flex-shrink: 0;
  }
  
  .user-info {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius);
    background: var(--bg-surface);
  }
  
  .user-info img {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .avatar-fallback {
    display: none;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--bg-hover);
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .avatar-fallback--visible {
    display: flex;
  }
  
  .user-info span {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
