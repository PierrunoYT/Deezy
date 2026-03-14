<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, onDestroy } from 'svelte';
  import { loggedIn, userInfo, activeDownloads, type UserInfo } from '$lib/stores';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import SearchView from '$lib/components/SearchView.svelte';
  import DownloadsView from '$lib/components/DownloadsView.svelte';
  import SettingsView from '$lib/components/SettingsView.svelte';
  import KeyboardShortcutsModal from '$lib/components/KeyboardShortcutsModal.svelte';
  import MiniPlayer from '$lib/components/MiniPlayer.svelte';
  import { keyboardShortcuts, type KeyboardShortcut } from '$lib/keyboardShortcuts';
  import { audioPlayerManager } from '$lib/audioPlayer';

  type ViewType = 'search' | 'downloads' | 'settings';

  let currentView = $state<ViewType>('search');
  let isLoggedIn = $state(false);
  let user = $state<UserInfo | null>(null);
  let activeCount = $state(0);
  let showShortcutsModal = $state(false);

  const SHORTCUT_IDS = [
    'view-search',
    'view-downloads', 
    'view-settings',
    'settings-shortcut',
    'help',
    'play-pause',
    'minimize-to-tray'
  ] as const;

  function isViewType(view: string): view is ViewType {
    return view === 'search' || view === 'downloads' || view === 'settings';
  }

  function switchView(view: string): void {
    if (!isViewType(view)) return;
    currentView = view;
  }

  async function minimizeToTray(): Promise<void> {
    try {
      const window = getCurrentWindow();
      await window.hide();
    } catch (err) {
      console.error('Failed to minimize to tray:', err);
    }
  }

  function registerShortcuts(): void {
    const shortcuts: Record<string, KeyboardShortcut> = {
      'view-search': {
        key: '1',
        ctrl: true,
        description: 'Switch to Search view',
        category: 'navigation',
        action: () => switchView('search')
      },
      'view-downloads': {
        key: '2',
        ctrl: true,
        description: 'Switch to Downloads view',
        category: 'navigation',
        action: () => switchView('downloads')
      },
      'view-settings': {
        key: '3',
        ctrl: true,
        description: 'Switch to Settings view',
        category: 'navigation',
        action: () => switchView('settings')
      },
      'settings-shortcut': {
        key: ',',
        ctrl: true,
        description: 'Open Settings',
        category: 'navigation',
        action: () => switchView('settings')
      },
      'help': {
        key: '?',
        shift: true,
        description: 'Show keyboard shortcuts',
        category: 'general',
        action: () => showShortcutsModal = true
      },
      'play-pause': {
        key: ' ',
        description: 'Play/Pause audio preview',
        category: 'general',
        action: () => audioPlayerManager.togglePlayPause()
      },
      'minimize-to-tray': {
        key: 'h',
        ctrl: true,
        description: 'Minimize to system tray',
        category: 'general',
        action: minimizeToTray
      }
    };

    Object.entries(shortcuts).forEach(([id, shortcut]) => {
      keyboardShortcuts.register(id, shortcut);
    });
  }

  function unregisterShortcuts(): void {
    SHORTCUT_IDS.forEach(id => {
      keyboardShortcuts.unregister(id);
    });
  }
  
  onMount(() => {
    const unsubscribe1 = loggedIn.subscribe(val => isLoggedIn = val);
    const unsubscribe2 = userInfo.subscribe(val => user = val);
    const unsubscribe3 = activeDownloads.subscribe(val => activeCount = val);

    if (!isLoggedIn) {
      currentView = 'settings';
    }

    registerShortcuts();
    keyboardShortcuts.attach();

    return () => {
      unsubscribe1();
      unsubscribe2();
      unsubscribe3();
      keyboardShortcuts.detach();
      unregisterShortcuts();
    };
  });

  onDestroy(() => {
    audioPlayerManager.destroy();
  });
</script>

<div id="app">
  <Sidebar 
    {currentView} 
    {user} 
    activeDownloads={activeCount}
    onViewChange={switchView}
    onShowHelp={() => showShortcutsModal = true}
  />
  
  <main id="content" aria-label="Main content">
    {#if currentView === 'search'}
      <SearchView />
    {:else if currentView === 'downloads'}
      <DownloadsView />
    {:else if currentView === 'settings'}
      <SettingsView onLoginSuccess={() => switchView('search')} />
    {/if}
  </main>
</div>

<MiniPlayer />

<KeyboardShortcutsModal 
  show={showShortcutsModal} 
  onClose={() => showShortcutsModal = false} 
/>

<style>
  #app {
    display: flex;
    height: 100vh;
  }
  
  #content {
    flex: 1;
    overflow-y: auto;
    padding: 0;
    padding-bottom: 80px;
  }
</style>
