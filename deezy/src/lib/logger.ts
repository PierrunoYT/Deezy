/**
 * Frontend side of the local diagnostic log.
 *
 * Lines go to the backend's `wavekeep-debug.log` through the `write_log`
 * command; nothing is sent anywhere else. The backend redacts each line before
 * writing it. Never log credentials (the ARL, tokens) or other secrets.
 */
import { invoke } from '@tauri-apps/api/core';

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

/** Turns anything thrown or logged into a single readable line. */
export function describe(value: unknown): string {
  if (value instanceof Error) {
    return value.stack?.split('\n').slice(0, 3).join(' | ') ?? `${value.name}: ${value.message}`;
  }
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}

/** Adds a line to the log. Never throws, and never logs its own failures. */
export function log(level: LogLevel, area: string, ...parts: unknown[]): void {
  const message = parts.map(describe).join(' ');
  void invoke('write_log', { level, area, message }).catch(() => {});
}

/**
 * Copies console.error / console.warn output and uncaught errors into the log,
 * so every existing error report in the UI ends up there. Returns a function
 * that undoes it.
 */
export function installErrorLogging(target: {
  console: Console;
  addEventListener: Window['addEventListener'];
  removeEventListener: Window['removeEventListener'];
}): () => void {
  const { console: con } = target;
  const originalError = con.error;
  const originalWarn = con.warn;

  con.error = (...args: unknown[]) => {
    originalError.apply(con, args);
    log('error', 'console', ...args);
  };
  con.warn = (...args: unknown[]) => {
    originalWarn.apply(con, args);
    log('warn', 'console', ...args);
  };

  const onError = (event: Event) => {
    const e = event as ErrorEvent;
    log('error', 'uncaught', e.error ?? e.message);
  };
  const onRejection = (event: Event) => {
    log('error', 'uncaught', 'Unhandled promise rejection:', (event as PromiseRejectionEvent).reason);
  };
  target.addEventListener('error', onError);
  target.addEventListener('unhandledrejection', onRejection);

  return () => {
    con.error = originalError;
    con.warn = originalWarn;
    target.removeEventListener('error', onError);
    target.removeEventListener('unhandledrejection', onRejection);
  };
}
