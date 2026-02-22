<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { loggedIn, userInfo, downloads, activeDownloads, downloadHistory, theme, currentLocale, type UserInfo, type DownloadItem, type Theme } from '$lib/stores';
  import { initI18n } from '$lib/i18n';
  import { locale as i18nLocale } from 'svelte-i18n';
  import { trayManager } from '$lib/tray';

  let { children } = $props();
  
  const MIN_SPLASH_MS = 2200;
  let appInitialized = $state(false);
  let uiVisible = $state(false);

  interface Settings {
    output_dir: string;
    quality: string;
    theme?: Theme;
    custom_theme?: string;
    locale?: string;
  }

  interface DownloadProgressEvent {
    track_id: string;
    title: string;
    percent: number;
    status: string;
  }

  async function applyTheme(themeValue: Theme) {
    const root = document.documentElement;
    
    if (themeValue === 'system') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      root.classList.toggle('light', !prefersDark);
    } else if (themeValue === 'custom') {
      try {
        const settings = await invoke<Settings>('get_settings');
        if (settings.custom_theme) {
          const themeData = await invoke<any>('load_custom_theme', { themeName: settings.custom_theme });
          const colors = themeData.colors;
          
          root.style.setProperty('--bg-darkest', colors['bg-darkest']);
          root.style.setProperty('--bg-dark', colors['bg-dark']);
          root.style.setProperty('--bg-surface', colors['bg-surface']);
          root.style.setProperty('--bg-elevated', colors['bg-elevated']);
          root.style.setProperty('--bg-hover', colors['bg-hover']);
          root.style.setProperty('--accent', colors.accent);
          root.style.setProperty('--accent-hover', colors['accent-hover']);
          root.style.setProperty('--accent-dim', colors['accent-dim']);
          root.style.setProperty('--text-primary', colors['text-primary']);
          root.style.setProperty('--text-secondary', colors['text-secondary']);
          root.style.setProperty('--text-tertiary', colors['text-tertiary']);
          root.style.setProperty('--success', colors.success);
          root.style.setProperty('--error', colors.error);
          root.style.setProperty('--warning', colors.warning);
          root.style.setProperty('--border', colors.border);
          
          root.classList.remove('light');
        }
      } catch (err) {
        console.error('Failed to load custom theme:', err);
        root.classList.toggle('light', false);
      }
    } else {
      root.classList.toggle('light', themeValue === 'light');
    }
  }
  
  onMount(() => {
    const splashStartedAt = Date.now();

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

    // Auto-login, load theme, and initialize i18n
    (async () => {
      try {
        const settings = await invoke<Settings>('get_settings');
        
        // Initialize i18n with saved locale
        const savedLocale = settings.locale || 'en';
        await initI18n(savedLocale);
        currentLocale.set(savedLocale);
        
        // Load theme preference
        if (settings.theme) {
          theme.set(settings.theme);
          applyTheme(settings.theme);
        } else {
          // Default to system theme
          theme.set('system');
          applyTheme('system');
        }
        
        try {
          const user = await invoke<UserInfo | null>('auto_login');
          if (user) {
            loggedIn.set(true);
            userInfo.set(user);
          }
        } catch (err) {
          console.error('Auto-login failed:', err);
        }
      } catch (err) {
        console.error('Failed to load settings:', err);
        // First run, initialize with default locale and system theme
        await initI18n('en');
        currentLocale.set('en');
        theme.set('system');
        applyTheme('system');
      } finally {
        const elapsed = Date.now() - splashStartedAt;
        const remaining = Math.max(0, MIN_SPLASH_MS - elapsed);
        if (remaining > 0) {
          await new Promise(resolve => setTimeout(resolve, remaining));
        }
        appInitialized = true;
        requestAnimationFrame(() => {
          uiVisible = true;
        });
      }
    })();
    
    // Initialize tray manager
    trayManager.init().catch(err => {
      console.error('Failed to initialize tray manager:', err);
    });
    
    // Listen for theme changes
    const unsubscribeTheme = theme.subscribe(applyTheme);

    // Listen for locale changes and update i18n
    const unsubscribeLocale = currentLocale.subscribe(newLocale => {
      i18nLocale.set(newLocale);
    });

    // Listen for system theme changes when in system mode
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleSystemThemeChange = () => {
      theme.update(t => {
        if (t === 'system') {
          applyTheme('system');
        }
        return t;
      });
    };
    mediaQuery.addEventListener('change', handleSystemThemeChange);

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
      unsubscribeTheme();
      unsubscribeLocale();
      mediaQuery.removeEventListener('change', handleSystemThemeChange);
      if (saveTimeout) clearTimeout(saveTimeout);
    };
  });
</script>

{#if appInitialized}
  <div class="app-shell" class:visible={uiVisible}>
    {@render children()}
  </div>
{:else}
  <div class="startup-splash" aria-live="polite" aria-busy="true">
    <img class="startup-logo" src="/logodeezy.svg" alt="Deezy logo" />
    <div class="startup-loading">
      <span class="spinner" aria-hidden="true"></span>
      <span>Loading Deezy...</span>
    </div>
  </div>
{/if}

<style>
  .app-shell {
    opacity: 0;
    transition: opacity 380ms ease;
  }

  .app-shell.visible {
    opacity: 1;
  }

  .startup-splash {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
    background: var(--bg-dark);
    color: var(--text-secondary);
  }

  .startup-logo {
    width: 120px;
    height: 120px;
    object-fit: contain;
    filter: drop-shadow(0 6px 24px rgba(162, 56, 255, 0.25));
  }

  .startup-loading {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
  }
</style>