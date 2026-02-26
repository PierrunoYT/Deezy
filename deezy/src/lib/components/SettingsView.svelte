<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { loggedIn, userInfo, theme, notificationsEnabled, searchHistory, currentLocale, type UserInfo, type Theme } from '$lib/stores';
  import { notificationManager } from '$lib/notifications';
  import { _, locale } from 'svelte-i18n';
  import { supportedLocales } from '$lib/i18n';
  import ThemeManager from './ThemeManager.svelte';
  
  interface Props {
    onLoginSuccess?: () => void;
  }
  
  let { onLoginSuccess }: Props = $props();
  
  let arl = $state('');
  let outputDir = $state('');
  let quality = $state('MP3_320');
  let folderStructure = $state('flat');
  let currentTheme = $state<Theme>('dark');
  let enableNotifications = $state(true);
  let enableSearchHistory = $state(true);
  let closeToTray = $state(true);
  let selectedLocale = $state('en');
  let showArl = $state(false);
  let saving = $state(false);
  let statusMsg = $state('');
  let statusType = $state<'success' | 'error' | 'info'>('info');
  let isFreeAccount = $derived(Boolean($userInfo?.is_free_account));
  let isLoggedIn = $state(false);

  const VALID_QUALITIES = ['MP3_128', 'MP3_320', 'FLAC'] as const;
  const MIN_ARL_LENGTH = 100;

  $effect(() => {
    if (isFreeAccount && quality !== 'MP3_128') {
      quality = 'MP3_128';
    }
  });
  
  onMount(async () => {
    try {
      const settings: any = await invoke('get_settings');
      if (settings.output_dir) outputDir = settings.output_dir;
      if (settings.quality) quality = settings.quality;
      if (settings.folder_structure) folderStructure = settings.folder_structure;
      if (settings.theme) currentTheme = settings.theme;
      if (settings.locale) selectedLocale = settings.locale;
      if (settings.notifications_enabled !== undefined) {
        enableNotifications = settings.notifications_enabled;
      }
      if (settings.enable_search_history !== undefined) {
        enableSearchHistory = settings.enable_search_history;
      }
      if (settings.close_to_tray !== undefined) {
        closeToTray = settings.close_to_tray;
      }
    } catch {
      // First run
    }

    // Initialize notification manager
    notificationManager.initialize();

    // Subscribe to theme changes
    const unsubTheme = theme.subscribe(t => currentTheme = t);
    
    // Subscribe to locale changes
    const unsubLocale = currentLocale.subscribe(l => selectedLocale = l);

    // Subscribe to logged in state
    const unsubLoggedIn = loggedIn.subscribe(val => isLoggedIn = val);

    return () => {
      unsubTheme();
      unsubLocale();
      unsubLoggedIn();
    };
  });
  
  async function pickFolder(): Promise<void> {
    try {
      const path = await invoke<string>('pick_folder');
      if (path) outputDir = path;
    } catch (err) {
      console.error('Folder picker failed:', err);
    }
  }

  function validateSettings(): { valid: boolean; error?: string } {
    const trimmedArl = arl.trim();

    if (trimmedArl && trimmedArl.length < MIN_ARL_LENGTH) {
      return { valid: false, error: $_('settings.status.arlInvalid') };
    }

    if (!outputDir.trim()) {
      return { valid: false, error: $_('settings.status.outputDirRequired') };
    }

    if (!VALID_QUALITIES.includes(quality as any)) {
      return { valid: false, error: $_('settings.status.qualityInvalid') };
    }

    return { valid: true };
  }
  
  async function saveSettings(): Promise<void> {
    const validation = validateSettings();
    if (!validation.valid) {
      showStatus(validation.error!, 'error');
      return;
    }

    const trimmedArl = arl.trim();
    saving = true;
    showStatus($_('settings.status.loggingIn'), 'info');

    try {
      await invoke('save_settings', {
        newSettings: {
          arl: trimmedArl,
          output_dir: outputDir.trim(),
          quality: quality,
          folder_structure: folderStructure,
          theme: currentTheme,
          notifications_enabled: enableNotifications,
          enable_search_history: enableSearchHistory,
          close_to_tray: closeToTray,
          locale: selectedLocale,
          search_history: []
        }
      });

      notificationManager.setEnabled(enableNotifications);
      notificationsEnabled.set(enableNotifications);

      const user = trimmedArl
        ? await invoke<UserInfo>('login', { arl: trimmedArl })
        : await invoke<UserInfo | null>('auto_login');

      if (!user) {
        showStatus($_('settings.status.arlRequired'), 'error');
        loggedIn.set(false);
        return;
      }

      loggedIn.set(true);
      userInfo.set(user);

      showStatus($_('settings.status.loginSuccess'), 'success');

      if (onLoginSuccess) {
        setTimeout(() => onLoginSuccess(), 1000);
      }
    } catch (err) {
      showStatus($_('settings.status.loginFailed', { values: { error: String(err) } }), 'error');
      loggedIn.set(false);
    } finally {
      saving = false;
    }
  }

  async function updateSetting<K extends string>(key: K, value: any): Promise<void> {
    try {
      const settings: any = await invoke('get_settings');
      await invoke('save_settings', {
        newSettings: {
          ...settings,
          [key]: value
        }
      });
    } catch (err) {
      console.error(`Failed to save ${key} setting:`, err);
      throw err;
    }
  }

  async function changeTheme(newTheme: Theme): Promise<void> {
    currentTheme = newTheme;
    theme.set(newTheme);
    
    try {
      await updateSetting('theme', newTheme);
    } catch (err) {
      console.error('Failed to save theme:', err);
    }
  }

  async function saveNotifications(): Promise<void> {
    notificationManager.setEnabled(enableNotifications);
    notificationsEnabled.set(enableNotifications);

    try {
      await updateSetting('notifications_enabled', enableNotifications);
    } catch (err) {
      console.error('Failed to save notification setting:', err);
    }
  }

  async function saveSearchHistory(): Promise<void> {
    try {
      await updateSetting('enable_search_history', enableSearchHistory);
    } catch (err) {
      console.error('Failed to save search history setting:', err);
    }
  }

  async function saveCloseToTray(): Promise<void> {
    try {
      await updateSetting('close_to_tray', closeToTray);
    } catch (err) {
      console.error('Failed to save close to tray setting:', err);
    }
  }

  async function clearSearchHistory(): Promise<void> {
    if (!confirm($_('settings.searchHistory.clearConfirm'))) {
      return;
    }

    try {
      await invoke('clear_search_history');
      searchHistory.set([]);
      showStatus($_('settings.status.historyCleared'), 'success');
      setTimeout(() => statusMsg = '', 3000);
    } catch (err) {
      showStatus($_('settings.status.clearHistoryFailed', { values: { error: String(err) } }), 'error');
    }
  }

  async function changeLocale(newLocale: string): Promise<void> {
    selectedLocale = newLocale;
    locale.set(newLocale);
    currentLocale.set(newLocale);
    
    try {
      await updateSetting('locale', newLocale);
    } catch (err) {
      console.error('Failed to save locale:', err);
    }
  }
  
  function showStatus(msg: string, type: 'success' | 'error' | 'info'): void {
    statusMsg = msg;
    statusType = type;
  }
  
</script>

<div class="view">
  <h2>{$_('settings.title')}</h2>
  
  <div class="settings-form">
    <div class="form-group">
      <label for="arl-input">{$_('settings.arl.label')}</label>
      <p class="form-hint">
        {$_('settings.arl.hint')}
      </p>
      {#if isLoggedIn && !arl}
        <div class="arl-saved-indicator">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
            <polyline points="22 4 12 14.01 9 11.01"/>
          </svg>
          <span>ARL token is securely saved (hidden for security)</span>
        </div>
      {/if}
      <div class="input-row">
        <input 
          type={showArl ? 'text' : 'password'}
          id="arl-input" 
          bind:value={arl}
          placeholder={isLoggedIn && !arl ? 'Enter new ARL to update' : $_('settings.arl.placeholder')}
        />
        <button class="btn-icon" onclick={() => showArl = !showArl} title={$_('settings.arl.showHide')}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
            <circle cx="12" cy="12" r="3"/>
          </svg>
        </button>
      </div>
    </div>
    
    <div class="form-group">
      <label for="output-input">{$_('settings.outputDir.label')}</label>
      <div class="input-row">
        <input 
          type="text" 
          id="output-input" 
          bind:value={outputDir}
          placeholder={$_('settings.outputDir.placeholder')}
        />
        <button class="btn-secondary" onclick={pickFolder}>{$_('settings.outputDir.browse')}</button>
      </div>
    </div>
    
    <div class="form-group">
      <label for="quality-select">{$_('settings.quality.label')}</label>
      <select id="quality-select" bind:value={quality}>
        <option value="MP3_128">{$_('settings.quality.mp3_128')}</option>
        <option value="MP3_320" disabled={isFreeAccount}>
          {$_('settings.quality.mp3_320')}{isFreeAccount ? ' (Premium)' : ''}
        </option>
        <option value="FLAC" disabled={isFreeAccount}>
          {$_('settings.quality.flac')}{isFreeAccount ? ' (Premium)' : ''}
        </option>
      </select>
      {#if isFreeAccount}
        <p class="form-hint">Your Deezer Free account supports MP3 128 kbps only.</p>
      {/if}
    </div>
    
    <div class="form-group">
      <label for="folder-structure-select">{$_('settings.folderStructure.label')}</label>
      <p class="form-hint">
        {$_('settings.folderStructure.hint')}
      </p>
      <select id="folder-structure-select" bind:value={folderStructure}>
        <option value="flat">{$_('settings.folderStructure.flat')}</option>
        <option value="artist_track">{$_('settings.folderStructure.artistTrack')}</option>
        <option value="artist_album_track">{$_('settings.folderStructure.artistAlbumTrack')}</option>
        <option value="album_track">{$_('settings.folderStructure.albumTrack')}</option>
      </select>
    </div>

    <div class="form-group">
      <div class="label-text">{$_('settings.theme.label')}</div>
      <p class="form-hint">
        {$_('settings.theme.hint')}
      </p>
      <div class="theme-selector">
        <button 
          class="theme-btn {currentTheme === 'light' ? 'active' : ''}"
          onclick={() => changeTheme('light')}
          type="button"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="5"/>
            <line x1="12" y1="1" x2="12" y2="3"/>
            <line x1="12" y1="21" x2="12" y2="23"/>
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
            <line x1="1" y1="12" x2="3" y2="12"/>
            <line x1="21" y1="12" x2="23" y2="12"/>
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
          </svg>
          {$_('settings.theme.light')}
        </button>
        <button 
          class="theme-btn {currentTheme === 'dark' ? 'active' : ''}"
          onclick={() => changeTheme('dark')}
          type="button"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
          </svg>
          {$_('settings.theme.dark')}
        </button>
        <button 
          class="theme-btn {currentTheme === 'system' ? 'active' : ''}"
          onclick={() => changeTheme('system')}
          type="button"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
            <line x1="8" y1="21" x2="16" y2="21"/>
            <line x1="12" y1="17" x2="12" y2="21"/>
          </svg>
          {$_('settings.theme.system')}
        </button>
      </div>
    </div>

    <div class="form-group">
      <div class="label-text">{$_('settings.language.label')}</div>
      <p class="form-hint">
        {$_('settings.language.hint')}
      </p>
      <select bind:value={selectedLocale} onchange={() => changeLocale(selectedLocale)}>
        {#each supportedLocales as loc}
          <option value={loc.code}>{loc.name}</option>
        {/each}
      </select>
    </div>

    <div class="form-group">
      <div class="label-text">{$_('settings.notifications.label')}</div>
      <p class="form-hint">
        {$_('settings.notifications.hint')}
      </p>
      <label class="toggle-wrapper">
        <span class="toggle-container">
          <input 
            type="checkbox" 
            bind:checked={enableNotifications}
            onchange={saveNotifications}
          />
          <span class="toggle-slider"></span>
        </span>
        <span class="toggle-label">
          {enableNotifications ? $_('settings.notifications.enabled') : $_('settings.notifications.disabled')}
        </span>
      </label>
    </div>

    <div class="form-group">
      <div class="label-text">{$_('settings.searchHistory.label')}</div>
      <p class="form-hint">
        {$_('settings.searchHistory.hint')}
      </p>
      <label class="toggle-wrapper">
        <span class="toggle-container">
          <input 
            type="checkbox" 
            bind:checked={enableSearchHistory}
            onchange={saveSearchHistory}
          />
          <span class="toggle-slider"></span>
        </span>
        <span class="toggle-label">
          {enableSearchHistory ? $_('settings.searchHistory.enabled') : $_('settings.searchHistory.disabled')}
        </span>
      </label>
      <button 
        class="btn-clear-history" 
        onclick={clearSearchHistory}
        type="button"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6"/>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
        </svg>
        {$_('settings.searchHistory.clear')}
      </button>
    </div>

    <div class="form-group">
      <div class="label-text">System Tray</div>
      <p class="form-hint">
        Minimize to system tray when closing the window instead of quitting the app.
      </p>
      <label class="toggle-wrapper">
        <span class="toggle-container">
          <input 
            type="checkbox" 
            bind:checked={closeToTray}
            onchange={saveCloseToTray}
          />
          <span class="toggle-slider"></span>
        </span>
        <span class="toggle-label">
          {closeToTray ? 'Close to Tray' : 'Close to Quit'}
        </span>
      </label>
    </div>

    <div class="form-actions">
      <button 
        class="btn-primary" 
        onclick={saveSettings}
        disabled={saving}
      >
        {saving ? $_('settings.actions.saving') : $_('settings.actions.save')}
      </button>
    </div>
    
    {#if statusMsg}
      <div class="status-message {statusType}">
        {statusMsg}
      </div>
    {/if}
  </div>
  
  <ThemeManager />
</div>

<style>
  .view {
    padding: 28px 32px;
    height: 100%;
    overflow-y: auto;
  }
  
  h2 {
    font-size: 24px;
    font-weight: 700;
    margin-bottom: 24px;
  }
  
  .settings-form {
    max-width: 560px;
  }
  
  .form-group {
    margin-bottom: 24px;
  }
  
  .form-group label,
  .form-group .label-text {
    display: block;
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 6px;
  }
  
  .form-hint {
    font-size: 12px;
    color: var(--text-tertiary);
    margin-bottom: 10px;
    line-height: 1.5;
  }

  .arl-saved-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    margin-bottom: 10px;
    border-radius: var(--radius);
    background: rgba(29, 185, 84, 0.1);
    border: 1px solid rgba(29, 185, 84, 0.2);
    color: var(--success);
    font-size: 13px;
    font-weight: 500;
  }

  .arl-saved-indicator svg {
    flex-shrink: 0;
  }
  
  .input-row {
    display: flex;
    gap: 8px;
  }
  
  .input-row input {
    flex: 1;
  }
  
  input[type="text"],
  input[type="password"] {
    width: 100%;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-primary);
    font-size: 14px;
    outline: none;
    transition: border-color 0.2s;
    font-family: inherit;
  }
  
  input:focus {
    border-color: var(--accent);
  }
  
  select {
    width: 100%;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-primary);
    font-size: 14px;
    outline: none;
    cursor: pointer;
    font-family: inherit;
    appearance: none;
    background-repeat: no-repeat;
    background-position: right 14px center;
  }

  :global(:root:not(.light)) select {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23666' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  }

  :global(:root.light) select {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23999' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  }
  
  select:focus {
    border-color: var(--accent);
  }
  
  .btn-primary {
    padding: 10px 28px;
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, transform 0.1s;
    font-family: inherit;
  }
  
  .btn-primary:hover {
    background: var(--accent-hover);
  }
  
  .btn-primary:active {
    transform: scale(0.98);
  }
  
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .btn-secondary {
    padding: 10px 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
    font-family: inherit;
    white-space: nowrap;
  }
  
  .btn-secondary:hover {
    background: var(--bg-hover);
  }
  
  .btn-icon {
    width: 40px;
    height: 40px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
    flex-shrink: 0;
  }
  
  .btn-icon:hover {
    background: var(--bg-hover);
  }

  .theme-selector {
    display: flex;
    gap: 8px;
  }

  .theme-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }

  .theme-btn:hover {
    background: var(--bg-hover);
    border-color: var(--text-tertiary);
  }

  .theme-btn.active {
    background: var(--accent-dim);
    border-color: var(--accent);
    color: var(--accent);
  }

  .theme-btn svg {
    flex-shrink: 0;
  }

  .toggle-wrapper {
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    user-select: none;
    width: fit-content;
  }

  .toggle-container {
    display: inline-block;
    position: relative;
    flex-shrink: 0;
  }

  .toggle-container input[type="checkbox"] {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    display: block;
    position: relative;
    width: 48px;
    height: 26px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 13px;
    transition: all 0.2s;
  }

  .toggle-slider::before {
    content: '';
    position: absolute;
    width: 18px;
    height: 18px;
    left: 3px;
    top: 3px;
    background: var(--text-tertiary);
    border-radius: 50%;
    transition: all 0.2s;
  }

  .toggle-container input:checked + .toggle-slider {
    background: var(--accent-dim);
    border-color: var(--accent);
  }

  .toggle-container input:checked + .toggle-slider::before {
    transform: translateX(22px);
    background: var(--accent);
  }

  .toggle-container:hover .toggle-slider {
    border-color: var(--text-tertiary);
  }

  .toggle-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .btn-clear-history {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }

  .btn-clear-history:hover {
    background: var(--bg-hover);
    border-color: var(--error);
    color: var(--error);
  }

  .btn-clear-history svg {
    flex-shrink: 0;
  }

  .form-actions {
    margin-top: 32px;
  }
  
  .status-message {
    margin-top: 16px;
    padding: 10px 14px;
    border-radius: var(--radius);
    font-size: 13px;
  }
  
  .status-message.success {
    background: rgba(29, 185, 84, 0.1);
    color: var(--success);
    border: 1px solid rgba(29, 185, 84, 0.2);
  }
  
  .status-message.error {
    background: rgba(231, 76, 60, 0.1);
    color: var(--error);
    border: 1px solid rgba(231, 76, 60, 0.2);
  }
  
  .status-message.info {
    background: var(--accent-dim);
    color: var(--accent);
    border: 1px solid rgba(162, 56, 255, 0.2);
  }
</style>
