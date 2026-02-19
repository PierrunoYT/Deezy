<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { loggedIn, userInfo, type UserInfo } from '$lib/stores';
  
  interface Props {
    onLoginSuccess?: () => void;
  }
  
  let { onLoginSuccess }: Props = $props();
  
  let arl = $state('');
  let outputDir = $state('');
  let quality = $state('MP3_320');
  let showArl = $state(false);
  let saving = $state(false);
  let statusMsg = $state('');
  let statusType = $state<'success' | 'error' | 'info'>('info');
  
  onMount(async () => {
    try {
      const settings: any = await invoke('get_settings');
      if (settings.arl) arl = settings.arl;
      if (settings.output_dir) outputDir = settings.output_dir;
      if (settings.quality) quality = settings.quality;
    } catch {
      // First run
    }
  });
  
  async function pickFolder() {
    try {
      const path = await invoke('pick_folder');
      if (path) outputDir = path as string;
    } catch (err) {
      console.error('Folder picker failed:', err);
    }
  }
  
  async function saveSettings() {
    // Validate ARL
    if (!arl.trim()) {
      showStatus('ARL token is required.', 'error');
      return;
    }

    if (arl.trim().length < 100) {
      showStatus('ARL token appears to be invalid (too short).', 'error');
      return;
    }

    // Validate output directory
    if (!outputDir.trim()) {
      showStatus('Download folder is required.', 'error');
      return;
    }

    // Validate quality
    const validQualities = ['MP3_128', 'MP3_320', 'FLAC'];
    if (!validQualities.includes(quality)) {
      showStatus('Invalid quality setting.', 'error');
      return;
    }

    saving = true;
    showStatus('Logging in to Deezer...', 'info');

    try {
      await invoke('save_settings', {
        newSettings: {
          arl: arl.trim(),
          output_dir: outputDir.trim(),
          quality: quality
        }
      });

      const user = await invoke<UserInfo>('login', { arl: arl.trim() });
      loggedIn.set(true);
      userInfo.set(user);

      showStatus('Logged in successfully!', 'success');

      if (onLoginSuccess) {
        setTimeout(() => onLoginSuccess(), 1000);
      }
    } catch (err) {
      showStatus(`Login failed: ${err}`, 'error');
      loggedIn.set(false);
    } finally {
      saving = false;
    }
  }
  
  function showStatus(msg: string, type: 'success' | 'error' | 'info') {
    statusMsg = msg;
    statusType = type;
  }
</script>

<div class="view">
  <h2>Settings</h2>
  
  <div class="settings-form">
    <div class="form-group">
      <label for="arl-input">ARL Token</label>
      <p class="form-hint">
        Log into deezer.com, open DevTools (F12), go to Application > Cookies, and copy the <code>arl</code> value.
      </p>
      <div class="input-row">
        <input 
          type={showArl ? 'text' : 'password'}
          id="arl-input" 
          bind:value={arl}
          placeholder="Paste your ARL token here..." 
        />
        <button class="btn-icon" onclick={() => showArl = !showArl} title="Show/hide">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
            <circle cx="12" cy="12" r="3"/>
          </svg>
        </button>
      </div>
    </div>
    
    <div class="form-group">
      <label for="output-input">Download Folder</label>
      <div class="input-row">
        <input 
          type="text" 
          id="output-input" 
          bind:value={outputDir}
          placeholder="Path to save downloads..." 
        />
        <button class="btn-secondary" onclick={pickFolder}>Browse</button>
      </div>
    </div>
    
    <div class="form-group">
      <label for="quality-select">Audio Quality</label>
      <select id="quality-select" bind:value={quality}>
        <option value="MP3_128">MP3 128 kbps</option>
        <option value="MP3_320">MP3 320 kbps</option>
        <option value="FLAC">FLAC (Lossless)</option>
      </select>
    </div>
    
    <div class="form-actions">
      <button 
        class="btn-primary" 
        onclick={saveSettings}
        disabled={saving}
      >
        {saving ? 'Connecting...' : 'Save & Login'}
      </button>
    </div>
    
    {#if statusMsg}
      <div class="status-message {statusType}">
        {statusMsg}
      </div>
    {/if}
  </div>
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
  
  .form-group label {
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
  
  .form-hint code {
    background: var(--bg-elevated);
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 12px;
    color: var(--accent);
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
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23666' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 14px center;
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
