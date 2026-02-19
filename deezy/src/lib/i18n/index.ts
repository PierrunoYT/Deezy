import { register, init, getLocaleFromNavigator, locale } from 'svelte-i18n';

// Register all locales
register('en', () => import('./locales/en.json'));
register('es', () => import('./locales/es.json'));
register('fr', () => import('./locales/fr.json'));
register('de', () => import('./locales/de.json'));

export const supportedLocales = [
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Español' },
  { code: 'fr', name: 'Français' },
  { code: 'de', name: 'Deutsch' }
];

let i18nInitialized = false;

function ensureI18nInitialized() {
  if (i18nInitialized) return;

  // Set a safe initial locale before first component render.
  init({
    fallbackLocale: 'en',
    initialLocale: 'en'
  });
  i18nInitialized = true;
}

ensureI18nInitialized();

export async function initI18n(savedLocale?: string) {
  ensureI18nInitialized();
  const targetLocale = savedLocale || getLocaleFromNavigator() || 'en';
  await Promise.resolve(locale.set(targetLocale));
}

export { locale };
