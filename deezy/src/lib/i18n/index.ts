import { register, init, getLocaleFromNavigator, locale, isLoading, waitLocale } from 'svelte-i18n';

export interface LocaleInfo {
  code: string;
  name: string;
  nativeName?: string;
}

const DEFAULT_LOCALE = 'en';
const FALLBACK_LOCALE = 'en';

export const supportedLocales: readonly LocaleInfo[] = [
  { code: 'en', name: 'English', nativeName: 'English' },
  { code: 'es', name: 'Spanish', nativeName: 'Español' },
  { code: 'fr', name: 'French', nativeName: 'Français' },
  { code: 'de', name: 'German', nativeName: 'Deutsch' },
  { code: 'pt', name: 'Portuguese', nativeName: 'Português' },
  { code: 'it', name: 'Italian', nativeName: 'Italiano' }
] as const;

const localeRegistry = {
  en: () => import('./locales/en.json'),
  es: () => import('./locales/es.json'),
  fr: () => import('./locales/fr.json'),
  de: () => import('./locales/de.json'),
  pt: () => import('./locales/pt.json'),
  it: () => import('./locales/it.json')
} as const;

let i18nInitialized = false;

function registerAllLocales(): void {
  Object.entries(localeRegistry).forEach(([code, loader]) => {
    register(code, loader);
  });
}

async function ensureI18nInitialized(): Promise<void> {
  if (i18nInitialized) return;

  registerAllLocales();

  await init({
    fallbackLocale: FALLBACK_LOCALE,
    initialLocale: DEFAULT_LOCALE
  });

  i18nInitialized = true;
}

export function isLocaleSupported(localeCode: string): boolean {
  return supportedLocales.some(loc => loc.code === localeCode);
}

export function getSupportedLocaleOrDefault(localeCode?: string | null): string {
  if (!localeCode) return DEFAULT_LOCALE;
  
  const normalizedCode = localeCode.toLowerCase().split('-')[0];
  return isLocaleSupported(normalizedCode) ? normalizedCode : DEFAULT_LOCALE;
}

export async function initI18n(savedLocale?: string): Promise<void> {
  await ensureI18nInitialized();
  
  const browserLocale = getLocaleFromNavigator();
  const targetLocale = getSupportedLocaleOrDefault(savedLocale || browserLocale);
  
  locale.set(targetLocale);
  await waitLocale(targetLocale);
}

export async function changeLocale(newLocale: string): Promise<void> {
  if (!isLocaleSupported(newLocale)) {
    console.warn(`Locale "${newLocale}" is not supported, falling back to ${DEFAULT_LOCALE}`);
    newLocale = DEFAULT_LOCALE;
  }

  locale.set(newLocale);
  await waitLocale(newLocale);
}

export function getCurrentLocale(): string {
  return locale.subscribe(val => val) as unknown as string;
}

export { locale, isLoading, waitLocale };
