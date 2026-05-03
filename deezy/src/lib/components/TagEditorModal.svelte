<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';

  interface Props {
    show: boolean;
    /** Pre-populate with a specific file path (e.g. from download history). */
    initialFilePath?: string | null;
    onClose: () => void;
  }

  interface FileTagData {
    file_path: string;
    format: string;
    title?: string;
    artist?: string;
    album?: string;
    album_artist?: string;
    year?: number;
    track?: number;
    total_tracks?: number;
    disc?: number;
    total_discs?: number;
    genre?: string;
    label?: string;
    comment?: string;
    cover_data?: string;
    cover_mime?: string;
  }

  let { show, initialFilePath = null, onClose }: Props = $props();

  // ── State ──────────────────────────────────────────────────────────────────
  let tags = $state<FileTagData | null>(null);
  let filePath = $state('');
  let loading = $state(false);
  let saving = $state(false);
  let error = $state('');
  let successMsg = $state('');

  // Editable fields (mirrors FileTagData minus file_path/format/cover_data/cover_mime)
  let title        = $state('');
  let artist       = $state('');
  let album        = $state('');
  let albumArtist  = $state('');
  let year         = $state('');
  let track        = $state('');
  let totalTracks  = $state('');
  let disc         = $state('');
  let totalDiscs   = $state('');
  let genre        = $state('');
  let label        = $state('');
  let comment      = $state('');

  // Cover art state
  let coverPreviewUrl = $state<string | null>(null);
  let newCoverPath    = $state<string | null>(null);
  let removeCover     = $state(false);

  let modalRef = $state<HTMLDivElement | undefined>(undefined);

  // ── Lifecycle ──────────────────────────────────────────────────────────────
  $effect(() => {
    if (show) {
      error = '';
      successMsg = '';
      if (initialFilePath && initialFilePath !== filePath) {
        filePath = initialFilePath;
        loadTagsFromPath(initialFilePath);
      }
      setTimeout(() => modalRef?.focus(), 0);
    } else {
      // Reset on close
      tags = null;
      filePath = '';
      resetFields();
    }
  });

  function resetFields(): void {
    title = artist = album = albumArtist = year = track = totalTracks = '';
    disc = totalDiscs = genre = label = comment = '';
    coverPreviewUrl = null;
    newCoverPath = null;
    removeCover = false;
    error = '';
    successMsg = '';
  }

  function populateFields(data: FileTagData): void {
    title       = data.title       ?? '';
    artist      = data.artist      ?? '';
    album       = data.album       ?? '';
    albumArtist = data.album_artist ?? '';
    year        = data.year        != null ? String(data.year) : '';
    track       = data.track       != null ? String(data.track) : '';
    totalTracks = data.total_tracks != null ? String(data.total_tracks) : '';
    disc        = data.disc        != null ? String(data.disc) : '';
    totalDiscs  = data.total_discs  != null ? String(data.total_discs) : '';
    genre       = data.genre       ?? '';
    label       = data.label       ?? '';
    comment     = data.comment     ?? '';

    newCoverPath = null;
    removeCover  = false;

    if (data.cover_data && data.cover_mime) {
      coverPreviewUrl = `data:${data.cover_mime};base64,${data.cover_data}`;
    } else {
      coverPreviewUrl = null;
    }
  }

  // ── Actions ────────────────────────────────────────────────────────────────
  async function pickFile(): Promise<void> {
    const picked = await invoke<string | null>('pick_audio_file');
    if (picked) {
      filePath = picked;
      await loadTagsFromPath(picked);
    }
  }

  async function loadTagsFromPath(path: string): Promise<void> {
    loading = true;
    error = '';
    successMsg = '';
    try {
      const data = await invoke<FileTagData>('read_file_tags', { filePath: path });
      tags = data;
      populateFields(data);
    } catch (e) {
      error = String(e);
      tags = null;
    } finally {
      loading = false;
    }
  }

  async function pickCover(): Promise<void> {
    const picked = await invoke<string | null>('pick_cover_image');
    if (!picked) return;
    newCoverPath = picked;
    removeCover = false;
    try {
      coverPreviewUrl = await invoke<string>('read_image_as_data_url', { filePath: picked });
    } catch (e) {
      // Preview failed; keep the path but show no image.
      console.error('Cover preview failed:', e);
      coverPreviewUrl = null;
    }
  }

  function clearCover(): void {
    newCoverPath = null;
    removeCover = true;
    coverPreviewUrl = null;
  }

  function resetCover(): void {
    newCoverPath = null;
    removeCover = false;
    if (tags?.cover_data && tags?.cover_mime) {
      coverPreviewUrl = `data:${tags.cover_mime};base64,${tags.cover_data}`;
    } else {
      coverPreviewUrl = null;
    }
  }

  async function save(): Promise<void> {
    if (!tags) return;
    saving = true;
    error = '';
    successMsg = '';

    try {
      await invoke('write_file_tags', {
        filePath: tags.file_path,
        tags: {
          title:        title       || null,
          artist:       artist      || null,
          album:        album       || null,
          album_artist: albumArtist || null,
          year:         year        ? parseInt(year, 10)        : null,
          track:        track       ? parseInt(track, 10)       : null,
          total_tracks: totalTracks ? parseInt(totalTracks, 10) : null,
          disc:         disc        ? parseInt(disc, 10)        : null,
          total_discs:  totalDiscs  ? parseInt(totalDiscs, 10)  : null,
          genre:        genre       || null,
          label:        label       || null,
          comment:      comment     || null,
          new_cover_path: newCoverPath ?? null,
          remove_cover:   removeCover,
        }
      });
      successMsg = $_('tagEditor.saveSuccess');
      // Refresh tags from disk
      await loadTagsFromPath(tags.file_path);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function handleBackdropClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) onClose();
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') { e.preventDefault(); onClose(); }
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }
</script>

{#if show}
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="tag-editor-title"
    bind:this={modalRef}
    tabindex="-1"
  >
    <div class="modal">
      <!-- Header -->
      <div class="modal-header">
        <h2 id="tag-editor-title">{$_('tagEditor.title')}</h2>
        <button class="close-btn" onclick={onClose} aria-label={$_('common.close')}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-content">

        <!-- File picker row -->
        <div class="file-row">
          <div class="file-info">
            {#if filePath}
              <span class="file-name" title={filePath}>{fileName(filePath)}</span>
              <span class="file-format">{tags?.format?.toUpperCase() ?? ''}</span>
            {:else}
              <span class="file-placeholder">{$_('tagEditor.noFileSelected')}</span>
            {/if}
          </div>
          <button class="btn btn-secondary" onclick={pickFile} disabled={loading || saving}>
            {filePath ? $_('tagEditor.changeFile') : $_('tagEditor.selectFile')}
          </button>
        </div>

        <!-- Status messages -->
        {#if loading}
          <div class="status-msg loading">{$_('common.loading')}</div>
        {/if}
        {#if error}
          <div class="status-msg error">{error}</div>
        {/if}
        {#if successMsg}
          <div class="status-msg success">{successMsg}</div>
        {/if}

        {#if tags}
          <div class="editor-body">
            <!-- Cover art column -->
            <div class="cover-col">
              <div class="cover-preview">
                {#if coverPreviewUrl}
                  <img src={coverPreviewUrl} alt={$_('tagEditor.coverAlt')} />
                  {#if newCoverPath}
                    <span class="cover-badge">{$_('tagEditor.newCoverSelected')}</span>
                  {/if}
                {:else}
                  <div class="cover-empty">
                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                      <rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/>
                      <polyline points="21 15 16 10 5 21"/>
                    </svg>
                    <span>{$_('tagEditor.noCover')}</span>
                  </div>
                {/if}
              </div>
              <div class="cover-actions">
                <button class="btn btn-secondary btn-sm" onclick={pickCover} disabled={saving}>
                  {$_('tagEditor.changeCover')}
                </button>
                {#if newCoverPath || coverPreviewUrl}
                  <button class="btn btn-ghost btn-sm" onclick={newCoverPath ? resetCover : clearCover} disabled={saving}>
                    {newCoverPath ? $_('tagEditor.cancelCoverChange') : $_('tagEditor.removeCover')}
                  </button>
                {/if}
              </div>
            </div>

            <!-- Tag fields grid -->
            <div class="fields-col">
              <div class="fields-grid">
                <label class="field field-wide">
                  <span>{$_('tagEditor.fields.title')}</span>
                  <input type="text" bind:value={title} disabled={saving} />
                </label>
                <label class="field field-wide">
                  <span>{$_('tagEditor.fields.artist')}</span>
                  <input type="text" bind:value={artist} disabled={saving} />
                </label>
                <label class="field field-wide">
                  <span>{$_('tagEditor.fields.album')}</span>
                  <input type="text" bind:value={album} disabled={saving} />
                </label>
                <label class="field field-wide">
                  <span>{$_('tagEditor.fields.albumArtist')}</span>
                  <input type="text" bind:value={albumArtist} disabled={saving} />
                </label>
                <label class="field">
                  <span>{$_('tagEditor.fields.year')}</span>
                  <input type="number" bind:value={year} min="1" max="9999" disabled={saving} />
                </label>
                <label class="field">
                  <span>{$_('tagEditor.fields.genre')}</span>
                  <input type="text" bind:value={genre} disabled={saving} />
                </label>
                <label class="field">
                  <span>{$_('tagEditor.fields.track')}</span>
                  <div class="inline-pair">
                    <input type="number" bind:value={track} min="1" placeholder="#" disabled={saving} />
                    <span class="of-label">/</span>
                    <input type="number" bind:value={totalTracks} min="1" placeholder="total" disabled={saving} />
                  </div>
                </label>
                <label class="field">
                  <span>{$_('tagEditor.fields.disc')}</span>
                  <div class="inline-pair">
                    <input type="number" bind:value={disc} min="1" placeholder="#" disabled={saving} />
                    <span class="of-label">/</span>
                    <input type="number" bind:value={totalDiscs} min="1" placeholder="total" disabled={saving} />
                  </div>
                </label>
                <label class="field field-wide">
                  <span>{$_('tagEditor.fields.label')}</span>
                  <input type="text" bind:value={label} disabled={saving} />
                </label>
                <label class="field field-wide">
                  <span>{$_('tagEditor.fields.comment')}</span>
                  <input type="text" bind:value={comment} disabled={saving} />
                </label>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      {#if tags}
        <div class="modal-footer">
          <button class="btn btn-ghost" onclick={onClose} disabled={saving}>
            {$_('common.cancel')}
          </button>
          <button class="btn btn-primary" onclick={save} disabled={saving || loading}>
            {saving ? $_('tagEditor.saving') : $_('common.save')}
          </button>
        </div>
      {/if}
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
    z-index: 1000;
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .modal {
    background: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 92%;
    max-width: 780px;
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s ease;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  @keyframes slideUp {
    from { transform: translateY(20px); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }

  /* ── Header ── */
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .modal-header h2 {
    font-size: 18px;
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
    flex-shrink: 0;
  }

  .close-btn:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  /* ── Body ── */
  .modal-content {
    padding: 20px 24px;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* File row */
  .file-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-elevated);
    border-radius: var(--radius);
    border: 1px solid var(--border);
  }

  .file-info {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .file-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-format {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-dim);
    padding: 2px 7px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .file-placeholder {
    font-size: 14px;
    color: var(--text-tertiary);
  }

  /* Status messages */
  .status-msg {
    padding: 10px 14px;
    border-radius: var(--radius);
    font-size: 13px;
  }

  .status-msg.loading {
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }

  .status-msg.error {
    background: rgba(239, 68, 68, 0.12);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.25);
  }

  .status-msg.success {
    background: rgba(34, 197, 94, 0.12);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.25);
  }

  /* Editor layout */
  .editor-body {
    display: flex;
    gap: 20px;
    align-items: flex-start;
  }

  /* Cover column */
  .cover-col {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex-shrink: 0;
    width: 160px;
  }

  .cover-preview {
    position: relative;
    width: 160px;
    height: 160px;
    border-radius: 8px;
    border: 1px solid var(--border);
    overflow: hidden;
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .cover-badge {
    position: absolute;
    bottom: 6px;
    left: 6px;
    right: 6px;
    background: var(--accent);
    color: white;
    font-size: 10px;
    font-weight: 600;
    text-align: center;
    padding: 3px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .cover-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cover-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--text-tertiary);
    font-size: 11px;
    text-align: center;
    padding: 8px;
  }

  .cover-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* Fields column */
  .fields-col {
    flex: 1;
    min-width: 0;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .field-wide {
    grid-column: span 2;
  }

  .field span {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-tertiary);
  }

  .field input {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    padding: 7px 10px;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
    box-sizing: border-box;
  }

  .field input:focus {
    border-color: var(--accent);
  }

  .field input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Remove number input spin buttons */
  .field input[type="number"]::-webkit-inner-spin-button,
  .field input[type="number"]::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .field input[type="number"] { -moz-appearance: textfield; }

  .inline-pair {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .inline-pair input {
    flex: 1;
    min-width: 0;
  }

  .of-label {
    color: var(--text-tertiary);
    font-size: 13px;
    flex-shrink: 0;
  }

  /* ── Footer ── */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  /* ── Buttons ── */
  .btn {
    padding: 8px 18px;
    border-radius: var(--radius);
    border: none;
    font-size: 13px;
    font-weight: 600;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:not(:disabled):hover {
    filter: brightness(1.1);
  }

  .btn-secondary {
    background: var(--bg-elevated);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .btn-secondary:not(:disabled):hover {
    background: var(--bg-hover);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
  }

  .btn-ghost:not(:disabled):hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-sm {
    padding: 6px 12px;
    font-size: 12px;
    width: 100%;
  }
</style>
