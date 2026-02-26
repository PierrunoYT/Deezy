import { get } from 'svelte/store';
import { locale } from 'svelte-i18n';

const DEFAULT_LOCALE = 'en';
const MILLION = 1_000_000;
const THOUSAND = 1_000;

function getCurrentLocale(): string {
  return get(locale) || DEFAULT_LOCALE;
}

export function formatDuration(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) {
    return '0:00';
  }

  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

export function formatNumber(value: number): string {
  if (!isFinite(value)) {
    return '0';
  }

  const currentLocale = getCurrentLocale();
  return new Intl.NumberFormat(currentLocale).format(value);
}

export function formatFans(count: number): string {
  if (!isFinite(count) || count < 0) {
    return '0';
  }

  if (count >= MILLION) {
    const millions = count / MILLION;
    return `${millions.toFixed(1)}M`;
  }

  if (count >= THOUSAND) {
    const thousands = Math.floor(count / THOUSAND);
    return `${thousands}K`;
  }

  return String(count);
}

export function formatDate(date: Date | string | number): string {
  const dateObj = date instanceof Date ? date : new Date(date);
  
  if (!isValidDate(dateObj)) {
    return '';
  }

  const currentLocale = getCurrentLocale();
  return new Intl.DateTimeFormat(currentLocale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  }).format(dateObj);
}

export function formatDateTime(date: Date | string | number): string {
  const dateObj = date instanceof Date ? date : new Date(date);
  
  if (!isValidDate(dateObj)) {
    return '';
  }

  const currentLocale = getCurrentLocale();
  return new Intl.DateTimeFormat(currentLocale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  }).format(dateObj);
}

export function formatTime(date: Date | string | number): string {
  const dateObj = date instanceof Date ? date : new Date(date);
  
  if (!isValidDate(dateObj)) {
    return '';
  }

  const currentLocale = getCurrentLocale();
  return new Intl.DateTimeFormat(currentLocale, {
    hour: '2-digit',
    minute: '2-digit'
  }).format(dateObj);
}

export function formatFileSize(bytes: number): string {
  if (!isFinite(bytes) || bytes < 0) {
    return '0 B';
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  const formatted = unitIndex === 0 ? size : size.toFixed(1);
  return `${formatted} ${units[unitIndex]}`;
}

function isValidDate(date: Date): boolean {
  return date instanceof Date && !isNaN(date.getTime());
}
