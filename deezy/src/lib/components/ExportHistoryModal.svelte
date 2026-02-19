<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';
  import type { DownloadItem } from '$lib/stores';

  interface Props {
    show: boolean;
    history: DownloadItem[];
    onClose: () => void;
  }

  let { show = $bindable(), history, onClose }: Props = $props();

  let selectedFormat = $state<'csv' | 'json'>('csv');
  let selectedDateRange = $state<'all' | 'week' | 'month' | 'year' | 'custom'>('all');
  let startDate = $state('');
  let endDate = $state('');
  let isExporting = $state(false);
  let exportMessage = $state<{ type: 'success' | 'error'; text: string } | null>(null);

  function filterHistoryByDateRange(items: DownloadItem[]): DownloadItem[] {
    if (selectedDateRange === 'all') {
      return items;
    }

    const now = new Date();
    let cutoffDate = new Date();

    switch (selectedDateRange) {
      case 'week':
        cutoffDate.setDate(now.getDate() - 7);
        break;
      case 'month':
        cutoffDate.setMonth(now.getMonth() - 1);
        break;
      case 'year':
        cutoffDate.setFullYear(now.getFullYear() - 1);
        break;
      case 'custom':
        if (startDate && endDate) {
          const start = new Date(startDate);
          const end = new Date(endDate);
          return items.filter(item => {
            if (!item.timestamp) return false;
            const itemDate = new Date(item.timestamp);
            return itemDate >= start && itemDate <= end;
          });
        }
        return items;
    }

    return items.filter(item => {
      if (!item.timestamp) return false;
      const itemDate = new Date(item.timestamp);
      return itemDate >= cutoffDate;
    });
  }

  async function handleExport() {
    if (history.length === 0) {
      exportMessage = { type: 'error', text: $_('downloads.export.emptyHistory') };
      return;
    }

    isExporting = true;
    exportMessage = null;

    try {
      const filteredHistory = filterHistoryByDateRange(history);
      
      if (filteredHistory.length === 0) {
        exportMessage = { type: 'error', text: $_('downloads.export.emptyHistory') };
        isExporting = false;
        return;
      }

      const result = await invoke<string>('export_download_history', {
        history: filteredHistory,
        format: selectedFormat
      });

      exportMessage = { 
        type: 'success', 
        text: $_('downloads.export.success', { values: { path: result } })
      };

      setTimeout(() => {
        onClose();
        exportMessage = null;
      }, 2000);
    } catch (err) {
      const errorMsg = String(err);
      if (errorMsg.includes('cancelled')) {
        exportMessage = { type: 'error', text: $_('downloads.export.cancelled') };
      } else {
        exportMessage = { 
          type: 'error', 
          text: $_('downloads.export.error', { values: { error: errorMsg } })
        };
      }
    } finally {
      isExporting = false;
    }
  }

  function handleClose() {
    if (!isExporting) {
      exportMessage = null;
      onClose();
    }
  }
</script>

{#if show}
  <div class="modal-overlay" onclick={handleClose}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>{$_('downloads.export.title')}</h3>
        <button class="close-btn" onclick={handleClose} disabled={isExporting}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <div class="modal-body">
        <div class="form-group">
          <label>{$_('downloads.export.format')}</label>
          <div class="radio-group">
            <label class="radio-option">
              <input type="radio" bind:group={selectedFormat} value="csv" disabled={isExporting} />
              <span>{$_('downloads.export.csv')}</span>
            </label>
            <label class="radio-option">
              <input type="radio" bind:group={selectedFormat} value="json" disabled={isExporting} />
              <span>{$_('downloads.export.json')}</span>
            </label>
          </div>
        </div>

        <div class="form-group">
          <label>{$_('downloads.export.dateRange')}</label>
          <select bind:value={selectedDateRange} disabled={isExporting}>
            <option value="all">{$_('downloads.export.allTime')}</option>
            <option value="week">{$_('downloads.export.lastWeek')}</option>
            <option value="month">{$_('downloads.export.lastMonth')}</option>
            <option value="year">{$_('downloads.export.lastYear')}</option>
            <option value="custom">{$_('downloads.export.custom')}</option>
          </select>
        </div>

        {#if selectedDateRange === 'custom'}
          <div class="date-range">
            <div class="form-group">
              <label>{$_('downloads.export.startDate')}</label>
              <input type="date" bind:value={startDate} disabled={isExporting} />
            </div>
            <div class="form-group">
              <label>{$_('downloads.export.endDate')}</label>
              <input type="date" bind:value={endDate} disabled={isExporting} />
            </div>
          </div>
        {/if}

        {#if exportMessage}
          <div class="message {exportMessage.type}">
            {exportMessage.text}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={handleClose} disabled={isExporting}>
          {$_('downloads.export.cancel')}
        </button>
        <button class="btn btn-primary" onclick={handleExport} disabled={isExporting}>
          {isExporting ? $_('downloads.export.exporting') : $_('downloads.export.exportButton')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }

  .modal {
    background: var(--bg-surface);
    border-radius: var(--radius-lg);
    width: 90%;
    max-width: 500px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    border: 1px solid var(--border);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h3 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .close-btn:hover:not(:disabled) {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .close-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .modal-body {
    padding: 24px;
  }

  .form-group {
    margin-bottom: 20px;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  .form-group label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }


  .radio-group {
    display: flex;
    gap: 12px;
  }

  .radio-option {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .radio-option:hover {
    background: var(--bg-elevated);
  }

  .radio-option input[type="radio"] {
    margin: 0;
    cursor: pointer;
  }

  .radio-option span {
    font-size: 14px;
    color: var(--text-primary);
  }

  select, input[type="date"] {
    width: 100%;
    padding: 10px 12px;
    font-size: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-primary);
    transition: all 0.15s ease;
  }

  select:focus, input[type="date"]:focus {
    outline: none;
    border-color: var(--accent);
  }

  select:disabled, input[type="date"]:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .date-range {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .message {
    padding: 12px 16px;
    border-radius: var(--radius);
    font-size: 13px;
    margin-top: 16px;
  }

  .message.success {
    background: rgba(34, 197, 94, 0.1);
    color: var(--success);
    border: 1px solid rgba(34, 197, 94, 0.2);
  }

  .message.error {
    background: rgba(239, 68, 68, 0.1);
    color: var(--error);
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .modal-footer {
    display: flex;
    gap: 12px;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
    justify-content: flex-end;
  }

  .btn {
    padding: 8px 16px;
    font-size: 14px;
    font-weight: 500;
    border-radius: var(--radius);
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }
</style>
