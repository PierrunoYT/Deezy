import { get } from 'svelte/store';
import { locale } from 'svelte-i18n';

export function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = String(seconds % 60).padStart(2, '0');
  return `${mins}:${secs}`;
}

export function formatNumber(n: number): string {
  const currentLocale = get(locale) || 'en';
  return new Intl.NumberFormat(currentLocale).format(n);
}

export function formatFans(n: number): string {
  if (n >= 1_000_000) {
    return `${(n / 1_000_000).toFixed(1)}M`;
  }
  if (n >= 1_000) {
    return `${Math.floor(n / 1_000)}K`;
  }
  return String(n);
}

export function formatDate(date: Date): string {
  const currentLocale = get(locale) || 'en';
  return new Intl.DateTimeFormat(currentLocale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  }).format(date);
}

export function formatDateTime(date: Date): string {
  const currentLocale = get(locale) || 'en';
  return new Intl.DateTimeFormat(currentLocale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  }).format(date);
}
