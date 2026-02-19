<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { theme } from '$lib/stores';
  
  interface ThemeColors {
    'bg-darkest': string;
    'bg-dark': string;
    'bg-surface': string;
    'bg-elevated': string;
    'bg-hover': string;
    accent: string;
    'accent-hover': string;
    'accent-dim': string;
    'text-primary': string;
    'text-secondary': string;
    'text-tertiary': string;
    success: string;
    error: string;
    warning: string;
    border: string;
  }
  
  interface CustomTheme {
    name: string;
    author?: string;
    description?: string;
    version: string;
    colors: ThemeColors;
  }
  
  let customThemes = $state<string[]>([]);
  let selectedTheme = $state<string | null>(null);
  let previewTheme = $state<CustomTheme | null>(null);
  let loading = $state(false);
  let statusMsg = $state('');
  let statusType = $state<'success' | 'error' | 'info'>('info');
  let showExportModal = $state(false);
  let exportName = $state('');
  let exportAuthor = $state('');
  let exportDescription = $state('');
  let exportAsLight = $state(false);
  
  onMount(async () => {
    await loadCustomThemes();
    await loadCurrentTheme();
  });
  
  async function loadCustomThemes() {
    try {
      customThemes = await invoke<string[]>('list_custom_themes');
    } catch (err) {
      console.error('Failed to load custom themes:', err);
    }
  }
  
  async function loadCurrentTheme() {
    try {
      const settings: any = await invoke('get_settings');
      if (settings.custom_theme) {
        selectedTheme = settings.custom_theme;
        await loadThemePreview(settings.custom_theme);
      }
    } catch (err) {
      console.error('Failed to load current theme:', err);
    }
  }
  
  async function loadThemePreview(themeName: string) {
    try {
      previewTheme = await invoke<CustomTheme>('load_custom_theme', { themeName });
    } catch (err) {
      console.error('Failed to load theme preview:', err);
      previewTheme = null;
    }
  }
  
  async function applyCustomTheme(themeName: string) {
    loading = true;
    try {
      const themeData = await invoke<CustomTheme>('load_custom_theme', { themeName });
      
      const root = document.documentElement;
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
      
      const settings: any = await invoke('get_settings');
      await invoke('save_settings', {
        newSettings: {
          ...settings,
          theme: 'custom',
          custom_theme: themeName
        }
      });
      
      theme.set('custom' as any);
      selectedTheme = themeName;
      
      showStatus(`Theme "${themeData.name}" applied successfully`, 'success');
    } catch (err) {
      showStatus(`Failed to apply theme: ${err}`, 'error');
    } finally {
      loading = false;
    }
  }
  
  async function resetToDefault() {
    const root = document.documentElement;
    
    root.style.removeProperty('--bg-darkest');
    root.style.removeProperty('--bg-dark');
    root.style.removeProperty('--bg-surface');
    root.style.removeProperty('--bg-elevated');
    root.style.removeProperty('--bg-hover');
    root.style.removeProperty('--accent');
    root.style.removeProperty('--accent-hover');
    root.style.removeProperty('--accent-dim');
    root.style.removeProperty('--text-primary');
    root.style.removeProperty('--text-secondary');
    root.style.removeProperty('--text-tertiary');
    root.style.removeProperty('--success');
    root.style.removeProperty('--error');
    root.style.removeProperty('--warning');
    root.style.removeProperty('--border');
    
    try {
      const settings: any = await invoke('get_settings');
      await invoke('save_settings', {
        newSettings: {
          ...settings,
          theme: 'dark',
          custom_theme: null
        }
      });
      
      theme.set('dark');
      selectedTheme = null;
      previewTheme = null;
      
      showStatus('Reset to default dark theme', 'success');
    } catch (err) {
      showStatus(`Failed to reset theme: ${err}`, 'error');
    }
  }
  
  async function deleteTheme(themeName: string) {
    if (!confirm(`Are you sure you want to delete the theme "${themeName}"?`)) {
      return;
    }
    
    try {
      await invoke('delete_custom_theme', { themeName });
      
      if (selectedTheme === themeName) {
        await resetToDefault();
      }
      
      await loadCustomThemes();
      showStatus(`Theme "${themeName}" deleted`, 'success');
    } catch (err) {
      showStatus(`Failed to delete theme: ${err}`, 'error');
    }
  }
  
  async function importTheme() {
    loading = true;
    try {
      const themeName = await invoke<string>('import_theme_file');
      await loadCustomThemes();
      showStatus(`Theme "${themeName}" imported successfully`, 'success');
    } catch (err) {
      if (err !== 'Import cancelled') {
        showStatus(`Failed to import theme: ${err}`, 'error');
      }
    } finally {
      loading = false;
    }
  }
  
  async function exportTheme() {
    if (!exportName.trim()) {
      showStatus('Please enter a theme name', 'error');
      return;
    }
    
    loading = true;
    try {
      const themeData = await invoke<CustomTheme>('export_current_theme', {
        themeName: exportName,
        author: exportAuthor || null,
        description: exportDescription || null,
        isLight: exportAsLight
      });
      
      await invoke('save_custom_theme', { theme: themeData });
      await loadCustomThemes();
      
      showStatus(`Theme "${exportName}" exported successfully`, 'success');
      showExportModal = false;
      exportName = '';
      exportAuthor = '';
      exportDescription = '';
      exportAsLight = false;
    } catch (err) {
      showStatus(`Failed to export theme: ${err}`, 'error');
    } finally {
      loading = false;
    }
  }
  
  async function createExamples() {
    loading = true;
    try {
      await invoke('create_example_themes');
      await loadCustomThemes();
      showStatus('Example themes created successfully', 'success');
    } catch (err) {
      showStatus(`Failed to create example themes: ${err}`, 'error');
    } finally {
      loading = false;
    }
  }
  
  function showStatus(msg: string, type: 'success' | 'error' | 'info') {
    statusMsg = msg;
    statusType = type;
    setTimeout(() => statusMsg = '', 3000);
  }
  
  function getColorPreview(colors: ThemeColors): string[] {
    return [
      colors['bg-dark'],
      colors['bg-elevated'],
      colors.accent,
      colors['text-primary'],
      colors.success
    ];
  }
</script>

<div class="theme-manager">
  <div class="manager-header">
    <h3>Custom Themes</h3>
    <div class="header-actions">
      <button class="btn-secondary" onclick={importTheme} disabled={loading}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
          <polyline points="7 10 12 15 17 10"/>
          <line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
        Import Theme
      </button>
      <button class="btn-secondary" onclick={() => showExportModal = true}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
          <polyline points="17 8 12 3 7 8"/>
          <line x1="12" y1="3" x2="12" y2="15"/>
        </svg>
        Export Current
      </button>
      <button class="btn-secondary" onclick={createExamples} disabled={loading}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="8" x2="12" y2="16"/>
          <line x1="8" y1="12" x2="16" y2="12"/>
        </svg>
        Add Examples
      </button>
    </div>
  </div>
  
  {#if customThemes.length === 0}
    <div class="empty-state">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/>
      </svg>
      <p>No custom themes yet</p>
      <p class="hint">Import a theme file or create example themes to get started</p>
    </div>
  {:else}
    <div class="themes-grid">
      {#each customThemes as themeName}
        {@const isActive = selectedTheme === themeName}
        <div class="theme-card {isActive ? 'active' : ''}" onmouseenter={() => loadThemePreview(themeName)}>
          <div class="theme-info">
            <h4>{themeName.replace(/_/g, ' ')}</h4>
            {#if previewTheme && previewTheme.name.toLowerCase().replace(/\s+/g, '_') === themeName}
              <p class="theme-description">{previewTheme.description || 'Custom theme'}</p>
              {#if previewTheme.author}
                <p class="theme-author">by {previewTheme.author}</p>
              {/if}
              <div class="color-preview">
                {#each getColorPreview(previewTheme.colors) as color}
                  <div class="color-swatch" style="background-color: {color}"></div>
                {/each}
              </div>
            {/if}
          </div>
          <div class="theme-actions">
            <button 
              class="btn-apply" 
              onclick={() => applyCustomTheme(themeName)}
              disabled={loading || isActive}
            >
              {isActive ? 'Active' : 'Apply'}
            </button>
            <button 
              class="btn-delete" 
              onclick={() => deleteTheme(themeName)}
              disabled={loading}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6"/>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
              </svg>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
  
  <div class="manager-footer">
    <button class="btn-reset" onclick={resetToDefault}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="1 4 1 10 7 10"/>
        <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/>
      </svg>
      Reset to Default Theme
    </button>
  </div>
  
  {#if statusMsg}
    <div class="status-message {statusType}">
      {statusMsg}
    </div>
  {/if}
</div>

{#if showExportModal}
  <div class="modal-overlay" onclick={() => showExportModal = false}>
    <div class="modal-content" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>Export Current Theme</h3>
        <button class="btn-close" onclick={() => showExportModal = false}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
      
      <div class="modal-body">
        <div class="form-group">
          <label for="theme-name">Theme Name *</label>
          <input 
            type="text" 
            id="theme-name" 
            bind:value={exportName}
            placeholder="My Awesome Theme"
          />
        </div>
        
        <div class="form-group">
          <label for="theme-author">Author</label>
          <input 
            type="text" 
            id="theme-author" 
            bind:value={exportAuthor}
            placeholder="Your name"
          />
        </div>
        
        <div class="form-group">
          <label for="theme-description">Description</label>
          <textarea 
            id="theme-description" 
            bind:value={exportDescription}
            placeholder="A brief description of your theme"
            rows="3"
          ></textarea>
        </div>
        
        <div class="form-group">
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={exportAsLight} />
            <span>Export as light theme</span>
          </label>
          <p class="form-hint">
            Check this if you want to export the default light theme colors instead of dark theme colors.
          </p>
        </div>
      </div>
      
      <div class="modal-footer">
        <button class="btn-secondary" onclick={() => showExportModal = false}>
          Cancel
        </button>
        <button class="btn-primary" onclick={exportTheme} disabled={loading}>
          {loading ? 'Exporting...' : 'Export Theme'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .theme-manager {
    margin-top: 32px;
    padding: 24px;
    background: var(--bg-surface);
    border-radius: var(--radius);
    border: 1px solid var(--border);
  }
  
  .manager-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }
  
  .manager-header h3 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }
  
  .header-actions {
    display: flex;
    gap: 8px;
  }
  
  .btn-secondary {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }
  
  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--accent);
    color: var(--accent);
  }
  
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .btn-secondary svg {
    flex-shrink: 0;
  }
  
  .empty-state {
    text-align: center;
    padding: 48px 24px;
    color: var(--text-secondary);
  }
  
  .empty-state svg {
    margin-bottom: 16px;
    opacity: 0.5;
  }
  
  .empty-state p {
    margin: 8px 0;
  }
  
  .empty-state .hint {
    font-size: 13px;
    color: var(--text-tertiary);
  }
  
  .themes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
  }
  
  .theme-card {
    padding: 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: all 0.2s;
  }
  
  .theme-card:hover {
    border-color: var(--accent);
    transform: translateY(-2px);
  }
  
  .theme-card.active {
    border-color: var(--accent);
    background: var(--accent-dim);
  }
  
  .theme-info h4 {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 8px 0;
    text-transform: capitalize;
  }
  
  .theme-description {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 4px 0;
  }
  
  .theme-author {
    font-size: 12px;
    color: var(--text-tertiary);
    margin: 4px 0 12px 0;
  }
  
  .color-preview {
    display: flex;
    gap: 6px;
    margin: 12px 0;
  }
  
  .color-swatch {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }
  
  .theme-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  
  .btn-apply {
    flex: 1;
    padding: 8px 16px;
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: white;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }
  
  .btn-apply:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  
  .btn-apply:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .btn-delete {
    width: 36px;
    height: 36px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }
  
  .btn-delete:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--error);
    color: var(--error);
  }
  
  .btn-delete:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .manager-footer {
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }
  
  .btn-reset {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }
  
  .btn-reset:hover {
    background: var(--bg-hover);
    border-color: var(--warning);
    color: var(--warning);
  }
  
  .btn-reset svg {
    flex-shrink: 0;
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
  
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }
  
  .modal-content {
    background: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    width: 90%;
    max-width: 480px;
    max-height: 90vh;
    overflow-y: auto;
  }
  
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
  }
  
  .modal-header h3 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }
  
  .btn-close {
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    transition: all 0.15s;
  }
  
  .btn-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .modal-body {
    padding: 24px;
  }
  
  .form-group {
    margin-bottom: 20px;
  }
  
  .form-group label {
    display: block;
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 8px;
  }
  
  .form-group input[type="text"],
  .form-group textarea {
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
    resize: vertical;
  }
  
  .form-group input:focus,
  .form-group textarea:focus {
    border-color: var(--accent);
  }
  
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    font-weight: 500;
  }
  
  .checkbox-label input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }
  
  .form-hint {
    font-size: 12px;
    color: var(--text-tertiary);
    margin-top: 6px;
    line-height: 1.5;
  }
  
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
  }
  
  .btn-primary {
    padding: 10px 24px;
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }
  
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
