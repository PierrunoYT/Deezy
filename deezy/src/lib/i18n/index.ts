import { register, init, getLocaleFromNavigator, locale, isLoading, waitLocale } from 'svelte-i18n';

// Register all locales
register('en', () => import('./locales/en.json'));
register('es', () => import('./locales/es.json'));
register('fr', () => import('./locales/fr.json'));
register('de', () => import('./locales/de.json'));
register('pt', () => import('./locales/pt.json'));
register('it', () => import('./locales/it.json'));

export const supportedLocales = [
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Español' },
  { code: 'fr', name: 'Français' },
  { code: 'de', name: 'Deutsch' },
  { code: 'pt', name: 'Português' },
  { code: 'it', name: 'Italiano' }
];

let i18nInitialized = false;

async function ensureI18nInitialized() {
  if (i18nInitialized) return;

  // Set a safe initial locale before first component render.
  await init({
    fallbackLocale: 'en',
    initialLocale: 'en'
  });
  i18nInitialized = true;
}

export async function initI18n(savedLocale?: string) {
  await ensureI18nInitialized();
  const targetLocale = savedLocale || getLocaleFromNavigator() || 'en';
  locale.set(targetLocale);
  await waitLocale(targetLocale);
}

export { locale, isLoading };
