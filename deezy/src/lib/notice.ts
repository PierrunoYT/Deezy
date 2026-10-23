/**
 * Startup warning and Info & FAQ content ids, plus the "don't show again" flag.
 *
 * The flag stores the notice version that was dismissed, so raising
 * `NOTICE_VERSION` after changing the warning's meaning shows it again.
 * Storage can be missing or throw (private windows, cleared site data), in
 * which case the notice is simply shown every time.
 */

export const NOTICE_VERSION = 1;
const STORAGE_KEY = 'wavekeep.startupNotice';

/** Items in the startup warning; text lives under `info.items.<id>` in the locale files. */
export const WARNING_ITEMS = ['legal', 'account', 'credentials', 'unofficial'] as const;

/** Items on the Info dialog's "Important" tab. */
export const IMPORTANT_ITEMS = [...WARNING_ITEMS, 'accounts', 'files', 'log'] as const;

/** FAQ entries; text lives under `info.faq.<id>.q` / `.a`. */
export const FAQ_ITEMS = [
  'deezy',
  'legal',
  'ban',
  'arl',
  'quality',
  'rateLimit',
  'twoLogins',
  'failed',
  'report',
  'tags',
  'privacy'
] as const;

function storage(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

export function shouldShowStartupNotice(): boolean {
  try {
    return storage()?.getItem(STORAGE_KEY) !== String(NOTICE_VERSION);
  } catch {
    return true;
  }
}

/** Remembers the dismissal only when the user ticked "don't show again". */
export function dismissStartupNotice(dontShowAgain: boolean): void {
  if (!dontShowAgain) return;
  try {
    storage()?.setItem(STORAGE_KEY, String(NOTICE_VERSION));
  } catch {
    // Without storage the notice just comes back next launch.
  }
}
