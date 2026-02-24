/**
 * Keyboard shortcuts handler for Deezy
 * Provides centralized keyboard shortcut management
 */

export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  cmd?: boolean;
  shift?: boolean;
  alt?: boolean;
  description: string;
  action: () => void;
  category: 'navigation' | 'search' | 'general';
}

export class KeyboardShortcutsManager {
  private shortcuts: Map<string, KeyboardShortcut> = new Map();
  private enabled = true;

  constructor() {
    this.handleKeyDown = this.handleKeyDown.bind(this);
  }

  /**
   * Register a keyboard shortcut
   */
  register(id: string, shortcut: KeyboardShortcut) {
    this.shortcuts.set(id, shortcut);
  }

  /**
   * Unregister a keyboard shortcut
   */
  unregister(id: string) {
    this.shortcuts.delete(id);
  }

  /**
   * Get all registered shortcuts grouped by category
   */
  getAllShortcuts(): Record<string, KeyboardShortcut[]> {
    const grouped: Record<string, KeyboardShortcut[]> = {
      navigation: [],
      search: [],
      general: []
    };

    this.shortcuts.forEach(shortcut => {
      grouped[shortcut.category].push(shortcut);
    });

    return grouped;
  }

  /**
   * Enable or disable keyboard shortcuts
   */
  setEnabled(enabled: boolean) {
    this.enabled = enabled;
  }

  /**
   * Handle keyboard events
   */
  handleKeyDown(event: KeyboardEvent) {
    if (!this.enabled) return;

    // Don't trigger shortcuts when typing in inputs (except for specific cases like Escape and Space)
    const target = event.target as HTMLElement;
    const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;

    // Allow Escape key and Space key even in inputs (Space only if not typing)
    if (event.key === 'Escape' && isInput) {
      // Let it pass through
    } else if (event.key === ' ' && isInput && target.tagName === 'INPUT') {
      // Don't handle spacebar in input fields
      return;
    } else if (isInput && event.key !== 'Escape' && event.key !== ' ') {
      // Don't handle other shortcuts when in input fields
      return;
    }

    const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const ctrlOrCmd = isMac ? event.metaKey : event.ctrlKey;

    // Find matching shortcut
    for (const [id, shortcut] of this.shortcuts) {
      const keyMatches = event.key.toLowerCase() === shortcut.key.toLowerCase();
      if (!keyMatches) continue;
      
      // Check modifiers - must match exactly what the shortcut requires
      const requiresCtrlOrCmd = shortcut.ctrl || shortcut.cmd;
      const requiresShift = shortcut.shift || false;
      const requiresAlt = shortcut.alt || false;
      
      const hasCtrlOrCmd = ctrlOrCmd;
      const hasShift = event.shiftKey;
      const hasAlt = event.altKey;
      
      // All modifiers must match exactly
      if (requiresCtrlOrCmd === hasCtrlOrCmd && 
          requiresShift === hasShift && 
          requiresAlt === hasAlt) {
        event.preventDefault();
        shortcut.action();
        return;
      }
    }
  }

  /**
   * Start listening for keyboard events
   */
  attach() {
    window.addEventListener('keydown', this.handleKeyDown);
  }

  /**
   * Stop listening for keyboard events
   */
  detach() {
    window.removeEventListener('keydown', this.handleKeyDown);
  }

  /**
   * Format shortcut for display
   */
  static formatShortcut(shortcut: KeyboardShortcut): string {
    const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
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

    // Format key name
    let keyName = shortcut.key.toUpperCase();
    if (keyName === 'ESCAPE') keyName = 'Esc';
    if (keyName === ',') keyName = ',';
    if (keyName === '?') keyName = '?';

    parts.push(keyName);

    return parts.join(isMac ? '' : '+');
  }
}

// Create a singleton instance
export const keyboardShortcuts = new KeyboardShortcutsManager();
