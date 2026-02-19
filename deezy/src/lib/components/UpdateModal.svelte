<script lang="ts">
  import { updateState } from '$lib/stores';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  
  interface Props {
    show: boolean;
    onClose: () => void;
  }
  
  let { show = $bindable(false), onClose }: Props = $props();
  
  let installing = $state(false);
  let installError = $state('');
  
  async function downloadAndInstall() {
    const state = $updateState;
    if (!state.updateInfo) return;
    
    installing = true;
    installError = '';
    
    try {
      updateState.update(s => ({ ...s, downloading: true, downloadProgress: 0 }));
      
      const update = await check();
      
      if (!update?.available) {
        throw new Error('Update is no longer available');
      }
      
      let totalBytes = 0;
      let downloadedBytes = 0;
      
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            totalBytes = event.data.contentLength || 0;
            downloadedBytes = 0;
            updateState.update(s => ({ 
              ...s, 
              downloading: true,
              downloadProgress: 0 
            }));
            break;
          case 'Progress':
            downloadedBytes += event.data.chunkLength || 0;
            const progress = totalBytes > 0 
              ? Math.round((downloadedBytes / totalBytes) * 100)
              : 0;
            updateState.update(s => ({ 
              ...s, 
              downloadProgress: progress
            }));
            break;
          case 'Finished':
            updateState.update(s => ({ 
              ...s, 
              downloading: false,
              downloadProgress: 100 
            }));
            break;
        }
      });
      
      await relaunch();
    } catch (err) {
      console.error('Update installation failed:', err);
      installError = err instanceof Error ? err.message : String(err);
      updateState.update(s => ({ 
        ...s, 
        downloading: false,
        error: installError 
      }));
    } finally {
      installing = false;
    }
  }
  
  function formatDate(dateStr?: string): string {
    if (!dateStr) return 'Unknown date';
    try {
      return new Date(dateStr).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
      });
    } catch {
      return dateStr;
    }
  }
  
  function handleClose() {
    if (!installing) {
      onClose();
    }
  }
</script>

{#if show}
  <div class="modal-overlay" onclick={handleClose} role="button" tabindex="-1">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h2>Update Available</h2>
        {#if !installing}
          <button class="close-btn" onclick={handleClose} aria-label="Close">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        {/if}
      </div>
      
      <div class="modal-content">
        {#if $updateState.updateInfo}
          <div class="update-info">
            <div class="version-badge">
              <span class="label">New Version:</span>
              <span class="version">{$updateState.updateInfo.version}</span>
            </div>
            <div class="version-badge current">
              <span class="label">Current:</span>
              <span class="version">{$updateState.updateInfo.currentVersion}</span>
            </div>
            {#if $updateState.updateInfo.date}
              <div class="release-date">
                Released on {formatDate($updateState.updateInfo.date)}
              </div>
            {/if}
          </div>
          
          {#if $updateState.updateInfo.body}
            <div class="release-notes">
              <h3>Release Notes</h3>
              <div class="notes-content">
                {$updateState.updateInfo.body}
              </div>
            </div>
          {/if}
          
          {#if $updateState.downloading}
            <div class="download-progress">
              <div class="progress-label">
                Downloading update... {$updateState.downloadProgress}%
              </div>
              <div class="progress-bar">
                <div class="progress-fill" style="width: {$updateState.downloadProgress}%"></div>
              </div>
            </div>
          {/if}
          
          {#if installError}
            <div class="error-message">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="8" x2="12" y2="12"/>
                <line x1="12" y1="16" x2="12.01" y2="16"/>
              </svg>
              {installError}
            </div>
          {/if}
        {/if}
      </div>
      
      <div class="modal-actions">
        <button 
          class="btn-secondary" 
          onclick={handleClose}
          disabled={installing}
        >
          Later
        </button>
        <button 
          class="btn-primary" 
          onclick={downloadAndInstall}
          disabled={installing}
        >
          {#if installing}
            {$updateState.downloading ? 'Downloading...' : 'Installing...'}
          {:else}
            Download & Install
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.2s ease-out;
  }
  
  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  
  .modal {
    background: var(--bg-primary);
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    width: 90%;
    max-width: 560px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.3s ease-out;
    border: 1px solid var(--border);
  }
  
  @keyframes slideUp {
    from {
      transform: translateY(20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }
  
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 24px 28px;
    border-bottom: 1px solid var(--border);
  }
  
  .modal-header h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0;
    color: var(--text-primary);
  }
  
  .close-btn {
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }
  
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .modal-content {
    padding: 24px 28px;
    overflow-y: auto;
    flex: 1;
  }
  
  .update-info {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 20px;
  }
  
  .version-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: var(--accent-dim);
    border: 1px solid var(--accent);
    border-radius: 8px;
  }
  
  .version-badge.current {
    background: var(--bg-elevated);
    border-color: var(--border);
  }
  
  .version-badge .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  
  .version-badge .version {
    font-size: 14px;
    font-weight: 700;
    color: var(--accent);
  }
  
  .version-badge.current .version {
    color: var(--text-primary);
  }
  
  .release-date {
    width: 100%;
    font-size: 13px;
    color: var(--text-tertiary);
    margin-top: 4px;
  }
  
  .release-notes {
    margin-bottom: 20px;
  }
  
  .release-notes h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 12px 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  
  .notes-content {
    padding: 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    white-space: pre-wrap;
    max-height: 200px;
    overflow-y: auto;
  }
  
  .download-progress {
    margin-bottom: 20px;
  }
  
  .progress-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
  }
  
  .progress-bar {
    height: 8px;
    background: var(--bg-elevated);
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid var(--border);
  }
  
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--accent-hover));
    transition: width 0.3s ease;
    border-radius: 4px;
  }
  
  .error-message {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: rgba(231, 76, 60, 0.1);
    border: 1px solid rgba(231, 76, 60, 0.3);
    border-radius: 8px;
    color: var(--error);
    font-size: 13px;
    margin-bottom: 20px;
  }
  
  .error-message svg {
    flex-shrink: 0;
  }
  
  .modal-actions {
    display: flex;
    gap: 12px;
    padding: 20px 28px;
    border-top: 1px solid var(--border);
  }
  
  .btn-primary,
  .btn-secondary {
    flex: 1;
    padding: 12px 24px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
    border: none;
  }
  
  .btn-primary {
    background: var(--accent);
    color: white;
  }
  
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(162, 56, 255, 0.3);
  }
  
  .btn-primary:active:not(:disabled) {
    transform: translateY(0);
  }
  
  .btn-secondary {
    background: var(--bg-elevated);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }
  
  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--text-tertiary);
  }
  
  .btn-primary:disabled,
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
