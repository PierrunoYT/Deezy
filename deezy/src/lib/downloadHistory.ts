import type { DownloadItem } from './stores';

// Work cannot remain active across process restarts. Preserve enough metadata
// to let the user resume interrupted downloads instead of dropping their rows.
export function recoverDownloadHistory(history: DownloadItem[]): DownloadItem[] {
  return history.map(item =>
    ['downloading', 'resolving', 'tagging'].includes(item.status)
      ? { ...item, status: 'paused', isPaused: true }
      : item
  );
}
