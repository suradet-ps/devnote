import { messages, type Locale, type MessageKey } from './translations';

/**
 * Minimal i18n helper (Roadmap Phase 3). Reactive: components reading `t()`
 * re-render when the locale changes.
 */

export type LocaleSetting = 'system' | Locale;

function detectLocale(): Locale {
  if (typeof navigator !== 'undefined') {
    const lang = (navigator.language ?? '').toLowerCase();
    if (lang.startsWith('th')) return 'th';
  }
  return 'en';
}

const localeState = $state<{ locale: Locale }>({ locale: detectLocale() });

export function getLocale(): Locale {
  return localeState.locale;
}

export function setLocale(locale: Locale): void {
  localeState.locale = locale;
}

/** Resolve a stored setting ('system' | 'en' | 'th') to an actual locale. */
export function resolveLocale(setting: LocaleSetting): Locale {
  return setting === 'system' ? detectLocale() : setting;
}

/**
 * Translate a key, substituting `{param}` placeholders.
 * Falls back to English, then to the key itself.
 */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const table = messages[localeState.locale] ?? messages.en;
  let out: string = table[key] ?? messages.en[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      out = out.replaceAll(`{${k}}`, String(v));
    }
  }
  return out;
}
