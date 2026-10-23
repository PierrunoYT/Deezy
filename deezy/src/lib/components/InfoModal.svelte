<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';
  import { IMPORTANT_ITEMS, FAQ_ITEMS } from '$lib/notice';

  interface Props {
    show: boolean;
    onClose: () => void;
  }

  let { show, onClose }: Props = $props();

  type Tab = 'important' | 'faq';

  let tab = $state<Tab>('important');
  let modalRef = $state<HTMLDivElement | undefined>(undefined);
  let logStatus = $state<'' | 'copied' | 'failed'>('');

  $effect(() => {
    if (show) {
      tab = 'important';
      logStatus = '';
      setTimeout(() => modalRef?.focus(), 0);
    }
  });

  /** Copies the recent, already-redacted log lines for a bug report. */
  async function copyRecentLog(): Promise<void> {
    try {
      const text = await invoke<string>('read_recent_log');
      await navigator.clipboard.writeText(text);
      logStatus = 'copied';
    } catch (err) {
      console.warn('Could not copy the log:', err);
      logStatus = 'failed';
    }
  }

  async function openLogFolder(): Promise<void> {
    try {
      await invoke('open_log_folder');
    } catch (err) {
      console.warn('Could not open the log folder:', err);
      logStatus = 'failed';
    }
  }

  function handleBackdropClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }
</script>

{#if show}
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="info-title"
    bind:this={modalRef}
    tabindex="-1"
  >
    <div class="modal">
      <div class="modal-header">
        <h2 id="info-title">{$_('info.title')}</h2>
        <button class="close-btn" onclick={onClose} aria-label={$_('info.close')}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <div class="tabs" role="tablist">
        <button
          class="tab"
          class:active={tab === 'important'}
          role="tab"
          aria-selected={tab === 'important'}
          onclick={() => tab = 'important'}
        >{$_('info.tabs.important')}</button>
        <button
          class="tab"
          class:active={tab === 'faq'}
          role="tab"
          aria-selected={tab === 'faq'}
          onclick={() => tab = 'faq'}
        >{$_('info.tabs.faq')}</button>
      </div>

      <div class="modal-content" role="tabpanel">
        {#if tab === 'important'}
          <p class="intro">{$_('info.warning.intro')}</p>
          <div class="item-list">
            {#each IMPORTANT_ITEMS as id (id)}
              <section class="item">
                <h3>{$_(`info.items.${id}.title`)}</h3>
                <p>{$_(`info.items.${id}.text`)}</p>
              </section>
            {/each}
          </div>
          <div class="log-actions">
            <button type="button" class="log-btn" onclick={copyRecentLog}>{$_('info.logActions.copy')}</button>
            <button type="button" class="log-btn" onclick={openLogFolder}>{$_('info.logActions.open')}</button>
          </div>
          {#if logStatus}
            <p class="log-status" class:failed={logStatus === 'failed'} role="status">{$_(`info.logActions.${logStatus}`)}</p>
          {/if}
        {:else}
          <div class="faq-list">
            {#each FAQ_ITEMS as id (id)}
              <details class="faq-item">
                <summary>{$_(`info.faq.${id}.q`)}</summary>
                <p>{$_(`info.faq.${id}.a`)}</p>
              </details>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .modal {
    background: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 90%;
    max-width: 680px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s ease;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  @keyframes slideUp {
    from { transform: translateY(20px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0;
  }

  .close-btn {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .tabs {
    display: flex;
    gap: 4px;
    padding: 12px 24px 0;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    padding: 8px 16px;
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .modal-content {
    padding: 20px 24px 24px;
    overflow-y: auto;
    flex: 1;
  }

  .intro {
    margin: 0 0 16px;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .item-list,
  .faq-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .item {
    padding: 12px 14px;
    background: var(--bg-elevated);
    border-radius: var(--radius);
  }

  .item h3 {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
  }

  .item p,
  .faq-item p {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-secondary);
  }

  .log-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
  }

  .log-btn {
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
  }

  .log-btn:hover {
    background: var(--bg-elevated);
  }

  .log-status {
    margin: 10px 0 0;
    font-size: 13px;
    color: var(--success);
  }

  .log-status.failed {
    color: var(--error);
  }

  .faq-item {
    background: var(--bg-elevated);
    border-radius: var(--radius);
    padding: 0 14px;
  }

  .faq-item summary {
    padding: 12px 0;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    list-style-position: inside;
  }

  .faq-item[open] summary {
    color: var(--accent);
  }

  .faq-item p {
    padding-bottom: 14px;
  }
</style>
