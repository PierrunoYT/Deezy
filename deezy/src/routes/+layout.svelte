<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    loggedIn,
    userInfo,
    downloads,
    downloadHistory,
    pausedDownloads,
    theme, 
    currentLocale, 
    type UserInfo, 
    type DownloadItem,
    type DownloadStatus,
    notificationsEnabled,
    type Theme,
    type AppSettings
  } from '$lib/stores';
  import { initI18n } from '$lib/i18n';
  import { locale as i18nLocale } from 'svelte-i18n';
  import { trayManager } from '$lib/tray';
  import { downloadQueueManager } from '$lib/downloadQueue';
  import { recoverDownloadHistory } from '$lib/downloadHistory';
  import { notificationManager } from '$lib/notifications';

  let { children } = $props();
  
  const MIN_SPLASH_MS = 2200;
  const HISTORY_SAVE_DELAY = 2000;
  
  let appInitialized = $state(false);
  let uiVisible = $state(false);

  interface DownloadProgressEvent {
    track_id: string;
    title: string;
    percent: number;
    status: DownloadStatus;
  }

  interface TagErrorEvent {
    track_id: string;
    error: string;
  }

  interface CustomThemeData {
    name: string;
    colors: Record<string, string>;
  }

  const CSS_VARIABLES = [
    'bg-darkest', 'bg-dark', 'bg-surface', 'bg-elevated', 'bg-hover',
    'accent', 'accent-hover', 'accent-dim',
    'text-primary', 'text-secondary', 'text-tertiary',
    'success', 'error', 'warning', 'border'
  ] as const;

  function applySystemTheme(): void {
    const root = document.documentElement;
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    root.classList.toggle('light', !prefersDark);
  }

  function applyCustomThemeColors(colors: Record<string, string>): void {
    const root = document.documentElement;
    CSS_VARIABLES.forEach(variable => {
      const value = colors[variable] || colors[variable.replace(/-/g, '_')];
      if (value) {
        root.style.setProperty(`--${variable}`, value);
      }
    });
    root.classList.remove('light');
  }

  let themeGeneration = 0;

  async function loadAndApplyCustomTheme(generation: number): Promise<void> {
    try {
      const settings = await invoke<AppSettings>('get_settings');
      if (!settings.custom_theme) return;

      const themeData = await invoke<CustomThemeData>('load_custom_theme', { 
        themeName: settings.custom_theme 
      });
      
      if (generation === themeGeneration) applyCustomThemeColors(themeData.colors);
    } catch (err) {
      console.error('Failed to load custom theme:', err);
      if (generation === themeGeneration) document.documentElement.classList.remove('light');
    }
  }

  async function applyTheme(themeValue: Theme): Promise<void> {
    const generation = ++themeGeneration;
    CSS_VARIABLES.forEach(variable => document.documentElement.style.removeProperty(`--${variable}`));
    if (themeValue === 'system') {
      applySystemTheme();
    } else if (themeValue === 'custom') {
      await loadAndApplyCustomTheme(generation);
    } else {
      const root = document.documentElement;
      root.classList.toggle('light', themeValue === 'light');
    }
  }
  
  let unlistenProgress: UnlistenFn | undefined;
  let unlistenTagError: UnlistenFn | undefined;
  let unlistenExitRequested: UnlistenFn | undefined;
  let saveHistoryTimeout: ReturnType<typeof setTimeout> | undefined;
  let pendingHistory: DownloadItem[] | undefined;
  let historySavePromise: Promise<void> | undefined;
  let exitInProgress = false;
  let mediaQuery: MediaQueryList | undefined;
  let systemThemeChangeHandler: (() => void) | undefined;

  async function loadDownloadHistory(): Promise<void> {
    try {
      const history = await invoke<DownloadItem[]>('load_download_history');
      if (history.length > 0) {
        const recovered = recoverDownloadHistory(history);
        downloadHistory.set(recovered);
        downloads.set(new Map(recovered.map(item => [item.trackId, item.status])));
        pausedDownloads.set(new Set(recovered.filter(item => item.status === 'paused').map(item => item.trackId)));
      }
    } catch (err) {
      console.error('Failed to load download history:', err);
    }
  }

  async function initializeApp(): Promise<void> {
    const splashStartedAt = Date.now();

    await loadDownloadHistory();

    try {
      const settings = await invoke<AppSettings>('get_settings');
      
      const notificationsOn = settings.notifications_enabled ?? true;
      notificationsEnabled.set(notificationsOn);
      notificationManager.setEnabled(notificationsOn);

      const savedLocale = settings.locale || 'en';
      await initI18n(savedLocale);
      currentLocale.set(savedLocale);
      
      const themeValue = settings.theme || 'system';
      theme.set(themeValue);
      await applyTheme(themeValue);
      
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
      notificationsEnabled.set(false);
      notificationManager.setEnabled(false);
      await initI18n('en');
      currentLocale.set('en');
      theme.set('system');
      await applyTheme('system');
    } finally {
      await ensureMinimumSplashTime(splashStartedAt);
      appInitialized = true;
      requestAnimationFrame(() => {
        uiVisible = true;
      });
    }
  }

  async function ensureMinimumSplashTime(startTime: number): Promise<void> {
    const elapsed = Date.now() - startTime;
    const remaining = Math.max(0, MIN_SPLASH_MS - elapsed);
    if (remaining > 0) {
      await new Promise(resolve => setTimeout(resolve, remaining));
    }
  }

  async function persistPendingDownloadHistory(): Promise<void> {
    while (pendingHistory) {
      const history = pendingHistory;
      pendingHistory = undefined;
      const toSave = recoverDownloadHistory(history);

      try {
        await invoke('save_download_history', { history: toSave });
      } catch (err) {
        console.error('Failed to save download history:', err);
      }
    }
  }

  function startHistorySave(): Promise<void> {
    if (!historySavePromise) {
      historySavePromise = persistPendingDownloadHistory().finally(() => {
        historySavePromise = undefined;
      });
    }

    return historySavePromise;
  }

  function saveDownloadHistory(history: DownloadItem[]): void {
    pendingHistory = history;

    if (saveHistoryTimeout) {
      clearTimeout(saveHistoryTimeout);
    }

    saveHistoryTimeout = setTimeout(() => {
      saveHistoryTimeout = undefined;
      void startHistorySave();
    }, HISTORY_SAVE_DELAY);
  }

  async function flushDownloadHistory(): Promise<void> {
    if (saveHistoryTimeout) {
      clearTimeout(saveHistoryTimeout);
      saveHistoryTimeout = undefined;
    }

    await startHistorySave();
  }

  async function handleExitRequested(): Promise<void> {
    if (exitInProgress) return;
    exitInProgress = true;

    await downloadQueueManager.prepareForExit();
    await flushDownloadHistory();
    await invoke('exit_app');
  }

  function handleDownloadProgress(event: DownloadProgressEvent): void {
    const { track_id, title, percent, status } = event;
    
    downloads.update(d => {
      d.set(track_id, status);
      return d;
    });

    downloadHistory.update(history => {
      const idx = history.findIndex(item => item.trackId === track_id);
      if (idx >= 0) {
        return history.map((item, i) =>
          i === idx ? { ...item, title, percent, status } : item
        );
      }
      return history;
    });
  }

  function setupSystemThemeListener(): void {
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    systemThemeChangeHandler = () => {
      theme.update(t => {
        if (t === 'system') {
          applyTheme('system');
        }
        return t;
      });
    };
    mediaQuery.addEventListener('change', systemThemeChangeHandler);
  }

  onMount(() => {
    let unsubscribeTheme = () => {};
    let unsubscribeLocale = () => {};
    let unsubscribeHistory = () => {};

    void (async () => {
      unlistenExitRequested = await listen('app-exit-requested', () => {
        void handleExitRequested().catch(err => {
          exitInProgress = false;
          console.error('Failed to exit cleanly:', err);
        });
      });

      await initializeApp();
      
      await trayManager.init().catch(err => {
        console.error('Failed to initialize tray manager:', err);
      });
      
      unsubscribeTheme = theme.subscribe(applyTheme);
      unsubscribeLocale = currentLocale.subscribe(newLocale => {
        i18nLocale.set(newLocale);
      });

      setupSystemThemeListener();

      let skipFirst = true;
      unsubscribeHistory = downloadHistory.subscribe((history) => {
        if (skipFirst) {
          skipFirst = false;
          return;
        }
        saveDownloadHistory(history);
      });

      unlistenProgress = await listen<DownloadProgressEvent>('download-progress', (event) => {
        handleDownloadProgress(event.payload);
      });

      unlistenTagError = await listen<TagErrorEvent>('tag-writing-error', (event) => {
        const { track_id, error } = event.payload;
        downloadHistory.update(history =>
          history.map(item =>
            item.trackId === track_id
              ? { ...item, errorMsg: `Warning: Tags not written - ${error}` }
              : item
          )
        );
      });
    })();

    return () => {
      unlistenProgress?.();
      unlistenTagError?.();
      unlistenExitRequested?.();
      unsubscribeHistory();
      unsubscribeTheme();
      unsubscribeLocale();
      if (mediaQuery && systemThemeChangeHandler) {
        mediaQuery.removeEventListener('change', systemThemeChangeHandler);
      }
      if (saveHistoryTimeout) clearTimeout(saveHistoryTimeout);
    };
  });

  onDestroy(() => {
    if (saveHistoryTimeout) {
      clearTimeout(saveHistoryTimeout);
    }
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
