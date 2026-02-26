<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { loggedIn, userInfo, downloads, activeDownloads, type UserInfo } from '$lib/stores';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import SearchView from '$lib/components/SearchView.svelte';
  import DownloadsView from '$lib/components/DownloadsView.svelte';
  import SettingsView from '$lib/components/SettingsView.svelte';
  import KeyboardShortcutsModal from '$lib/components/KeyboardShortcutsModal.svelte';
  import MiniPlayer from '$lib/components/MiniPlayer.svelte';
  import { keyboardShortcuts } from '$lib/keyboardShortcuts';
  import { audioPlayerManager } from '$lib/audioPlayer';

  let currentView = $state('search');
  let isLoggedIn = $state<boolean>(false);
  let user = $state<UserInfo | null>(null);
  let activeCount = $state<number>(0);
  let showShortcutsModal = $state(false);
  
  onMount(() => {
    // Subscribe to stores
    const unsubscribe1 = loggedIn.subscribe(val => isLoggedIn = val);
    const unsubscribe2 = userInfo.subscribe(val => user = val);
    const unsubscribe3 = activeDownloads.subscribe(val => activeCount = val);

    if (!isLoggedIn) {
      currentView = 'settings';
    }

    // Register keyboard shortcuts
    keyboardShortcuts.register('view-search', {
      key: '1',
      ctrl: true,
      description: 'Switch to Search view',
      category: 'navigation',
      action: () => switchView('search')
    });

    keyboardShortcuts.register('view-downloads', {
      key: '2',
      ctrl: true,
      description: 'Switch to Downloads view',
      category: 'navigation',
      action: () => switchView('downloads')
    });

    keyboardShortcuts.register('view-settings', {
      key: '3',
      ctrl: true,
      description: 'Switch to Settings view',
      category: 'navigation',
      action: () => switchView('settings')
    });

    keyboardShortcuts.register('settings-shortcut', {
      key: ',',
      ctrl: true,
      description: 'Open Settings',
      category: 'navigation',
      action: () => switchView('settings')
    });

    keyboardShortcuts.register('help', {
      key: '?',
      shift: true,
      description: 'Show keyboard shortcuts',
      category: 'general',
      action: () => showShortcutsModal = true
    });

    keyboardShortcuts.register('play-pause', {
      key: ' ',
      description: 'Play/Pause audio preview',
      category: 'general',
      action: () => audioPlayerManager.togglePlayPause()
    });

    keyboardShortcuts.register('minimize-to-tray', {
      key: 'h',
      ctrl: true,
      description: 'Minimize to system tray',
      category: 'general',
      action: async () => {
        const window = getCurrentWindow();
        await window.hide();
      }
    });

    // Attach keyboard listener
    keyboardShortcuts.attach();

    // Cleanup
    return () => {
      unsubscribe1();
      unsubscribe2();
      unsubscribe3();
      keyboardShortcuts.detach();
      keyboardShortcuts.unregister('view-search');
      keyboardShortcuts.unregister('view-downloads');
      keyboardShortcuts.unregister('view-settings');
      keyboardShortcuts.unregister('settings-shortcut');
      keyboardShortcuts.unregister('help');
      keyboardShortcuts.unregister('play-pause');
      keyboardShortcuts.unregister('minimize-to-tray');
    };
  });
  
  function switchView(view: string) {
    currentView = view;
  }
</script>

<div id="app">
  <Sidebar 
    {currentView} 
    {user} 
    activeDownloads={activeCount}
    onViewChange={switchView}
    onShowHelp={() => showShortcutsModal = true}
  />
  
  <main id="content">
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

<KeyboardShortcutsModal show={showShortcutsModal} onClose={() => showShortcutsModal = false} />

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
