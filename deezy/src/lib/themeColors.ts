/**
 * Checks a custom theme colour before it becomes a CSS variable.
 *
 * Theme files can come from other people, so a value is only used when it is
 * a plain colour: no `;` or braces that could add declarations, no `url(` or
 * other functions that load or substitute content, and (in a browser) a value
 * the engine itself accepts as a `color`. Environments without `CSS.supports`
 * (the tests) rely on the character checks alone.
 */

export const MAX_THEME_COLOR_LENGTH = 100;

const COLOR_CHARACTERS = /^[#a-z0-9(),.%\s/+-]+$/i;
const DISALLOWED_FUNCTIONS = /(?:url|var|env|attr|image|image-set|element|cross-fade|paint)\s*\(/i;

export function isSafeThemeColor(value: unknown): value is string {
  if (typeof value !== 'string') return false;
  const color = value.trim();
  if (!color || color.length > MAX_THEME_COLOR_LENGTH) return false;
  if (!COLOR_CHARACTERS.test(color) || DISALLOWED_FUNCTIONS.test(color)) return false;
  const css = (globalThis as { CSS?: { supports?: (property: string, value: string) => boolean } }).CSS;
  if (typeof css?.supports === 'function') {
    return css.supports('color', color);
  }
  return true;
}
