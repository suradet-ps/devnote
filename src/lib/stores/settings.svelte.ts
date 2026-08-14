import type { Store } from '@tauri-apps/plugin-store';

export interface Settings {
  theme: 'light' | 'dark' | 'system';
  fontSize: number;
  fontFamily: string;
  wordWrap: boolean;
  showLineNumbers: boolean;
  showStatusBar: boolean;
  tabSize: number;
  insertSpaces: boolean;
}

const DEFAULT_SETTINGS: Settings = {
  theme: 'system',
  fontSize: 14,
  fontFamily: 'JetBrains Mono',
  wordWrap: false,
  showLineNumbers: true,
  showStatusBar: true,
  tabSize: 4,
  insertSpaces: true,
};

const LOCALSTORAGE_KEY = 'devnote-settings';
// Historical keys we migrate from on first load. Newest first: if both keys
// exist, the current key wins over the stale legacy one.
const LEGACY_KEYS = [LOCALSTORAGE_KEY, 'sabot-settings'] as const;

/**
 * Pick only known, type-checked keys from untrusted persisted data.
 * Unknown keys and out-of-range values fall back to defaults instead of
 * leaking into the live settings object.
 */
function sanitizeSettings(partial: unknown): Partial<Settings> {
  const src = (partial ?? {}) as Record<string, unknown>;
  const out: Partial<Settings> = {};
  if (src.theme === 'light' || src.theme === 'dark' || src.theme === 'system') {
    out.theme = src.theme;
  }
  if (typeof src.fontSize === 'number' && src.fontSize >= 8 && src.fontSize <= 32) {
    out.fontSize = src.fontSize;
  }
  if (typeof src.fontFamily === 'string' && src.fontFamily.length > 0) {
    out.fontFamily = src.fontFamily;
  }
  if (typeof src.wordWrap === 'boolean') out.wordWrap = src.wordWrap;
  if (typeof src.showLineNumbers === 'boolean') out.showLineNumbers = src.showLineNumbers;
  if (typeof src.showStatusBar === 'boolean') out.showStatusBar = src.showStatusBar;
  if (typeof src.tabSize === 'number' && src.tabSize >= 1 && src.tabSize <= 16) {
    out.tabSize = src.tabSize;
  }
  if (typeof src.insertSpaces === 'boolean') out.insertSpaces = src.insertSpaces;
  return out;
}

function resolveSettings(raw: unknown): Settings {
  return { ...DEFAULT_SETTINGS, ...sanitizeSettings(raw) };
}

let _settings = $state<Settings>({ ...DEFAULT_SETTINGS });
let _initialized = false;
let _store: Store | null = null;
let _mediaQuery: MediaQueryList | null = null;
let _mediaQueryHandler: ((e: MediaQueryListEvent) => void) | null = null;
let _themeVersion = $state(0);

function detachMediaQuery(): void {
  if (_mediaQuery && _mediaQueryHandler) {
    _mediaQuery.removeEventListener('change', _mediaQueryHandler);
  }
  _mediaQuery = null;
  _mediaQueryHandler = null;
}

async function initStore(): Promise<void> {
  if (_initialized) return;

  // Prefer the Tauri store; fall back to localStorage
  try {
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load('.settings.dat');
    _store = store;
    const fromStore = await store.get<Partial<Settings>>('settings');
    if (fromStore) {
      _settings = resolveSettings(fromStore);
      _initialized = true;
      return;
    }
    // Migration: if the Tauri store is empty, look in localStorage
    for (const key of LEGACY_KEYS) {
      const raw = localStorage.getItem(key);
      if (raw) {
        try {
          const parsed = JSON.parse(raw) as Partial<Settings>;
          _settings = resolveSettings(parsed);
          // Persist to the Tauri store so future loads are consistent
          await store.set('settings', _settings);
          await store.save();
          // Best-effort cleanup of legacy localStorage key
          if (key === 'sabot-settings') localStorage.removeItem(key);
          break;
        } catch {
          // ignore malformed JSON
        }
      }
    }
  } catch (e) {
    console.warn('Tauri store unavailable, using localStorage:', e);
    for (const key of LEGACY_KEYS) {
      const raw = localStorage.getItem(key);
      if (raw) {
        try {
          _settings = resolveSettings(JSON.parse(raw) as Partial<Settings>);
          break;
        } catch {
          // ignore
        }
      }
    }
  }
  _initialized = true;
}

async function persistSettings(): Promise<void> {
  try {
    if (_store) {
      await _store.set('settings', _settings);
      await _store.save();
    } else {
      localStorage.setItem(LOCALSTORAGE_KEY, JSON.stringify(_settings));
    }
  } catch (e) {
    console.warn('Failed to persist settings:', e);
  }
}

export const settingsStore = {
  get settings() {
    return _settings;
  },
  get theme() {
    return _settings.theme;
  },
  get fontSize() {
    return _settings.fontSize;
  },
  get wordWrap() {
    return _settings.wordWrap;
  },
  get showLineNumbers() {
    return _settings.showLineNumbers;
  },
  get showStatusBar() {
    return _settings.showStatusBar;
  },
  get tabSize() {
    return _settings.tabSize;
  },
  get insertSpaces() {
    return _settings.insertSpaces;
  },
  get initialized() {
    return _initialized;
  },
  get themeVersion() {
    return _themeVersion;
  },

  async init(): Promise<void> {
    await initStore();
    this.applySystemTheme();
  },

  getEffectiveTheme(): 'light' | 'dark' {
    if (_settings.theme === 'system') {
      if (typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches) {
        return 'dark';
      }
      return 'light';
    }
    return _settings.theme;
  },

  applySystemTheme(): void {
    if (typeof window === 'undefined') return;

    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    detachMediaQuery();

    const handler = (): void => {
      if (_settings.theme === 'system') {
        document.documentElement.setAttribute('data-theme', mq.matches ? 'dark' : 'light');
        _themeVersion++;
      }
    };
    _mediaQuery = mq;
    _mediaQueryHandler = handler;
    mq.addEventListener('change', handler);

    document.documentElement.setAttribute(
      'data-theme',
      _settings.theme === 'system' ? (mq.matches ? 'dark' : 'light') : _settings.theme,
    );
  },

  update(partial: Partial<Settings>): void {
    _settings = { ..._settings, ...partial };
    _themeVersion++;
    this.applySystemTheme();
    void persistSettings();
  },

  increaseFontSize(): void {
    this.update({ fontSize: Math.min(_settings.fontSize + 1, 32) });
  },

  decreaseFontSize(): void {
    this.update({ fontSize: Math.max(_settings.fontSize - 1, 8) });
  },

  resetFontSize(): void {
    this.update({ fontSize: 14 });
  },

  toggleWordWrap(): void {
    this.update({ wordWrap: !_settings.wordWrap });
  },

  toggleStatusBar(): void {
    this.update({ showStatusBar: !_settings.showStatusBar });
  },

  /**
   * Test-only helper to reset store state. Not part of the public API.
   * @internal
   */
  __resetForTests(): void {
    _settings = { ...DEFAULT_SETTINGS };
    _initialized = false;
    _store = null;
  },
};
