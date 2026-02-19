<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { loggedIn, userInfo, downloads, activeDownloads, type UserInfo } from '$lib/stores';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import SearchView from '$lib/components/SearchView.svelte';
  import DownloadsView from '$lib/components/DownloadsView.svelte';
  import SettingsView from '$lib/components/SettingsView.svelte';

  let currentView = $state('search');
  let isLoggedIn = $state<boolean>(false);
  let user = $state<UserInfo | null>(null);
  let activeCount = $state<number>(0);

  // Use idiomatic Svelte 5 pattern with proper cleanup
  $effect(() => {
    try {
      const unsubscribe1 = loggedIn.subscribe(val => isLoggedIn = val);
      const unsubscribe2 = userInfo.subscribe(val => user = val);
      const unsubscribe3 = activeDownloads.subscribe(val => activeCount = val);
      return () => {
        unsubscribe1();
        unsubscribe2();
        unsubscribe3();
      };
    } catch (err) {
      console.error('Error in effect:', err);
    }
  });
  
  onMount(() => {
    if (!isLoggedIn) {
      currentView = 'settings';
    }
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

<style>
  #app {
    display: flex;
    height: 100vh;
  }
  
  #content {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }
</style>
