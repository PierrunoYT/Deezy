<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { loggedIn, userInfo, downloads, activeDownloads } from '$lib/stores';
  
  let { children } = $props();
  
  onMount(async () => {
    try {
      const settings: any = await invoke('get_settings');
      if (settings.arl) {
        try {
          console.log('Auto-logging in with saved ARL...');
          const user = await invoke('login', { arl: settings.arl });
          loggedIn.set(true);
          userInfo.set(user);
          console.log('Auto-login successful:', user);
        } catch (err) {
          console.error('Auto-login failed:', err);
          // ARL expired or invalid
        }
      }
    } catch (err) {
      console.error('Failed to load settings:', err);
      // First run, no settings yet
    }

    // Listen for download progress and cleanup on unmount
    const unlisten = await listen('download-progress', (event: any) => {
      const { track_id, status } = event.payload;
      downloads.update(d => {
        d.set(track_id, status);
        // Calculate active downloads directly without creating new subscription
        const active = Array.from(d.values()).filter(s => s === 'downloading').length;
        activeDownloads.set(active);
        return d;
      });
    });

    // Cleanup event listener on unmount
    return () => {
      unlisten();
    };
  });
</script>

{@render children()}