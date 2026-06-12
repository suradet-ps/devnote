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

let _settings = $state<Settings>({ ...DEFAULT_SETTINGS });
let _initialized = false;
let _store: any = null;

async function initStore() {
  if (_initialized) return;
  try {
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load('.settings.dat');
    _store = store;
    const stored = await store.get<Partial<Settings>>('settings');
    if (stored) {
      _settings = { ...DEFAULT_SETTINGS, ...stored };
    }
  } catch {
    // Fallback to localStorage
    try {
      const raw = localStorage.getItem('text-rs-settings');
      if (raw) _settings = { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
    } catch {}
  }
  _initialized = true;
}

async function persistSettings() {
  try {
    if (_store) {
      await _store.set('settings', _settings);
      await _store.save();
    } else {
      localStorage.setItem('text-rs-settings', JSON.stringify(_settings));
    }
  } catch {}
}

let _mediaQueryHandler: ((e: MediaQueryListEvent) => void) | null = null;
let _themeVersion = $state(0);

export const settingsStore = {
  get settings() { return _settings; },
  get theme() { return _settings.theme; },
  get fontSize() { return _settings.fontSize; },
  get wordWrap() { return _settings.wordWrap; },
  get showLineNumbers() { return _settings.showLineNumbers; },
  get showStatusBar() { return _settings.showStatusBar; },
  get tabSize() { return _settings.tabSize; },
  get insertSpaces() { return _settings.insertSpaces; },
  get initialized() { return _initialized; },
  get themeVersion() { return _themeVersion; },

  async init() {
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

  applySystemTheme() {
    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(prefers-color-scheme: dark)');

      if (_mediaQueryHandler) {
        mq.removeEventListener('change', _mediaQueryHandler);
      }

      const handler = () => {
        if (_settings.theme === 'system') {
          document.documentElement.setAttribute('data-theme', mq.matches ? 'dark' : 'light');
          _themeVersion++;
        }
      };
      _mediaQueryHandler = handler;
      mq.addEventListener('change', handler);

      document.documentElement.setAttribute('data-theme', _settings.theme);
      if (_settings.theme === 'system') {
        document.documentElement.setAttribute('data-theme', mq.matches ? 'dark' : 'light');
      }
    }
  },

  update(partial: Partial<Settings>) {
    _settings = { ..._settings, ...partial };
    _themeVersion++;
    this.applySystemTheme();
    persistSettings();
  },

  increaseFontSize() {
    this.update({ fontSize: Math.min(_settings.fontSize + 1, 32) });
  },

  decreaseFontSize() {
    this.update({ fontSize: Math.max(_settings.fontSize - 1, 8) });
  },

  resetFontSize() {
    this.update({ fontSize: 14 });
  },

  toggleWordWrap() {
    this.update({ wordWrap: !_settings.wordWrap });
  },

  toggleLineNumbers() {
    this.update({ showLineNumbers: !_settings.showLineNumbers });
  },

  toggleStatusBar() {
    this.update({ showStatusBar: !_settings.showStatusBar });
  },
};
