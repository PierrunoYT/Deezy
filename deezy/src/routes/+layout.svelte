<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { loggedIn, userInfo, downloads, activeDownloads, downloadHistory, type UserInfo, type DownloadItem } from '$lib/stores';

  let { children } = $props();

  interface Settings {
    arl: string;
    output_dir: string;
    quality: string;
  }

  interface DownloadProgressEvent {
    track_id: string;
    title: string;
    percent: number;
    status: string;
  }
  
  onMount(() => {
    // Load persisted download history
    (async () => {
      try {
        const history = await invoke<DownloadItem[]>('load_download_history');
        if (history.length > 0) {
          downloadHistory.set(history);
        }
      } catch (err) {
        console.error('Failed to load download history:', err);
      }
    })();

    // Auto-login
    (async () => {
      try {
        const settings = await invoke<Settings>('get_settings');
        if (settings.arl) {
          try {
            console.log('Auto-logging in with saved ARL...');
            const user = await invoke<UserInfo>('login', { arl: settings.arl });
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
    })();

    // Debounce-save download history on changes
    let saveTimeout: ReturnType<typeof setTimeout> | undefined;
    let skipFirst = true;
    const unsubscribe = downloadHistory.subscribe((history) => {
      // Skip the initial subscription call (or the load we just did)
      if (skipFirst) {
        skipFirst = false;
        return;
      }
      if (saveTimeout) clearTimeout(saveTimeout);
      saveTimeout = setTimeout(() => {
        const toSave = history.filter(item => item.status !== 'downloading');
        invoke('save_download_history', { history: toSave }).catch(err =>
          console.error('Failed to save download history:', err)
        );
      }, 2000);
    });

    // Listen for download progress and cleanup on unmount
    let unlisten: (() => void) | undefined;

    listen<DownloadProgressEvent>('download-progress', (event) => {
      const { track_id, title, percent, status } = event.payload;
      downloads.update(d => {
        d.set(track_id, status);
        // Calculate active downloads directly without creating new subscription
        const active = Array.from(d.values()).filter(s => s === 'downloading').length;
        activeDownloads.set(active);
        return d;
      });

      // Update download history so the progress bar works even when
      // DownloadsView is not mounted
      downloadHistory.update(history => {
        const idx = history.findIndex(item => item.trackId === track_id);
        if (idx >= 0) {
          return history.map((item, i) =>
            i === idx ? { ...item, title, percent, status } : item
          );
        }
        return history;
      });
    }).then(fn => {
      unlisten = fn;
    });

    // Cleanup event listener and subscription on unmount
    return () => {
      if (unlisten) {
        unlisten();
      }
      unsubscribe();
      if (saveTimeout) clearTimeout(saveTimeout);
    };
  });
</script>

{@render children()}