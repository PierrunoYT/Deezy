/**
 * Keyboard shortcuts handler for Deezy
 * Provides centralized keyboard shortcut management
 */

export type ShortcutCategory = 'navigation' | 'search' | 'general';

export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  cmd?: boolean;
  shift?: boolean;
  alt?: boolean;
  description: string;
  action: () => void;
  category: ShortcutCategory;
}

interface ModifierState {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
}

const INPUT_TAGS = ['INPUT', 'TEXTAREA'] as const;
const ALLOWED_KEYS_IN_INPUT = ['Escape', ' '] as const;

export class KeyboardShortcutsManager {
  private shortcuts = new Map<string, KeyboardShortcut>();
  private enabled = true;

  constructor() {
    this.handleKeyDown = this.handleKeyDown.bind(this);
  }

  register(id: string, shortcut: KeyboardShortcut): void {
    this.shortcuts.set(id, shortcut);
  }

  unregister(id: string): boolean {
    return this.shortcuts.delete(id);
  }

  getAllShortcuts(): Record<ShortcutCategory, KeyboardShortcut[]> {
    const grouped: Record<ShortcutCategory, KeyboardShortcut[]> = {
      navigation: [],
      search: [],
      general: []
    };

    this.shortcuts.forEach(shortcut => {
      grouped[shortcut.category].push(shortcut);
    });

    return grouped;
  }

  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
  }

  getEnabled(): boolean {
    return this.enabled;
  }

  clear(): void {
    this.shortcuts.clear();
  }

  has(id: string): boolean {
    return this.shortcuts.has(id);
  }

  getShortcut(id: string): KeyboardShortcut | undefined {
    return this.shortcuts.get(id);
  }

  private isMac(): boolean {
    return navigator.platform.toUpperCase().includes('MAC');
  }

  private isInputElement(target: HTMLElement): boolean {
    return INPUT_TAGS.includes(target.tagName as any) || target.isContentEditable;
  }

  private shouldIgnoreInInput(event: KeyboardEvent, target: HTMLElement): boolean {
    if (!this.isInputElement(target)) return false;

    // Always let Escape through so shortcuts like "close modal" still work.
    if (event.key === 'Escape') return false;

    // Ignore all other keys (including Space) so that typing in any text
    // field — INPUT, TEXTAREA, or contentEditable — is never hijacked.
    return true;
  }

  private getModifierState(event: KeyboardEvent): ModifierState {
    const ctrlOrCmd = this.isMac() ? event.metaKey : event.ctrlKey;
    
    return {
      ctrl: ctrlOrCmd,
      shift: event.shiftKey,
      alt: event.altKey
    };
  }

  private matchesShortcut(event: KeyboardEvent, shortcut: KeyboardShortcut): boolean {
    const keyMatches = event.key.toLowerCase() === shortcut.key.toLowerCase();
    if (!keyMatches) return false;

    const modifiers = this.getModifierState(event);
    const requiresCtrlOrCmd = shortcut.ctrl || shortcut.cmd || false;
    const requiresShift = shortcut.shift || false;
    const requiresAlt = shortcut.alt || false;

    return (
      requiresCtrlOrCmd === modifiers.ctrl &&
      requiresShift === modifiers.shift &&
      requiresAlt === modifiers.alt
    );
  }

  handleKeyDown(event: KeyboardEvent): void {
    if (!this.enabled) return;

    const target = event.target as HTMLElement;
    
    if (this.shouldIgnoreInInput(event, target)) {
      return;
    }

    for (const [_, shortcut] of this.shortcuts) {
      if (this.matchesShortcut(event, shortcut)) {
        event.preventDefault();
        shortcut.action();
        return;
      }
    }
  }

  attach(): void {
    if (typeof window !== 'undefined') {
      window.addEventListener('keydown', this.handleKeyDown);
    }
  }

  detach(): void {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', this.handleKeyDown);
    }
  }

  static formatShortcut(shortcut: KeyboardShortcut): string {
    const isMac = navigator.platform.toUpperCase().includes('MAC');
    const parts: string[] = [];

    if (shortcut.ctrl || shortcut.cmd) {
      parts.push(isMac ? '⌘' : 'Ctrl');
    }
    if (shortcut.shift) {
      parts.push(isMac ? '⇧' : 'Shift');
    }
    if (shortcut.alt) {
      parts.push(isMac ? '⌥' : 'Alt');
    }

    const keyName = KeyboardShortcutsManager.formatKeyName(shortcut.key);
    parts.push(keyName);

    return parts.join(isMac ? '' : '+');
  }

  private static formatKeyName(key: string): string {
    const keyMap: Record<string, string> = {
      'ESCAPE': 'Esc',
      'ARROWUP': '↑',
      'ARROWDOWN': '↓',
      'ARROWLEFT': '←',
      'ARROWRIGHT': '→',
      ' ': 'Space'
    };

    const upperKey = key.toUpperCase();
    return keyMap[upperKey] ?? upperKey;
  }
}

export const keyboardShortcuts = new KeyboardShortcutsManager();
