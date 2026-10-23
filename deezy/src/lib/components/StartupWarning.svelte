<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { WARNING_ITEMS } from '$lib/notice';

  interface Props {
    show: boolean;
    /** Called when the warning is dismissed; `dontShowAgain` reflects the checkbox. */
    onClose: (dontShowAgain: boolean) => void;
    onMoreInfo: () => void;
  }

  let { show, onClose, onMoreInfo }: Props = $props();

  let dontShowAgain = $state(false);
  let modalRef = $state<HTMLDivElement | undefined>(undefined);

  $effect(() => {
    if (show) {
      dontShowAgain = false;
      setTimeout(() => modalRef?.focus(), 0);
    }
  });

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose(dontShowAgain);
    }
  }
</script>

{#if show}
  <div
    class="modal-backdrop"
    onkeydown={handleKeydown}
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="warning-title"
    aria-describedby="warning-intro"
    bind:this={modalRef}
    tabindex="-1"
  >
    <div class="modal">
      <div class="modal-header">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        <h2 id="warning-title">{$_('info.warning.title')}</h2>
      </div>

      <div class="modal-content">
        <p id="warning-intro" class="intro">{$_('info.warning.intro')}</p>
        <ul class="items">
          {#each WARNING_ITEMS as id (id)}
            <li>
              <strong>{$_(`info.items.${id}.title`)}</strong>
              <span>{$_(`info.items.${id}.text`)}</span>
            </li>
          {/each}
        </ul>
      </div>

      <div class="modal-footer">
        <label class="dont-show">
          <input type="checkbox" bind:checked={dontShowAgain} />
          <span>{$_('info.warning.dontShow')}</span>
        </label>
        <div class="actions">
          <button class="btn-secondary" type="button" onclick={onMoreInfo}>
            {$_('info.warning.moreInfo')}
          </button>
          <button class="btn-primary" type="button" onclick={() => onClose(dontShowAgain)}>
            {$_('info.warning.understand')}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
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
    max-width: 560px;
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
    gap: 12px;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
    color: var(--warning);
  }

  .modal-header h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0;
    color: var(--text-primary);
  }

  .modal-content {
    padding: 20px 24px;
    overflow-y: auto;
    flex: 1;
  }

  .intro {
    margin: 0 0 14px;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .items li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 14px;
    background: var(--bg-elevated);
    border-left: 3px solid var(--warning);
    border-radius: var(--radius-sm);
  }

  .items strong {
    font-size: 14px;
    font-weight: 600;
  }

  .items span {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
  }

  .dont-show {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .btn-primary,
  .btn-secondary {
    padding: 8px 18px;
    border-radius: var(--radius);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-primary {
    border: 1px solid var(--accent);
    background: var(--accent);
    color: #fff;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .btn-secondary {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
  }

  .btn-secondary:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
</style>
