export interface Settings {
  theme: 'light' | 'dark' | 'system';
  fontSize: number;
  fontFamily: string;
  wordWrap: boolean;
  showLineNumbers: boolean;
  tabSize: number;
  insertSpaces: boolean;
}

const DEFAULT_SETTINGS: Settings = {
  theme: 'light',
  fontSize: 14,
  fontFamily: 'JetBrains Mono',
  wordWrap: false,
  showLineNumbers: true,
  tabSize: 2,
  insertSpaces: true,
};

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem('sabot-settings');
    if (raw) return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {}
  return { ...DEFAULT_SETTINGS };
}

function saveSettings(settings: Settings) {
  localStorage.setItem('sabot-settings', JSON.stringify(settings));
}

let _settings = $state<Settings>(loadSettings());

export const settingsStore = {
  get settings() { return _settings; },
  get theme() { return _settings.theme; },
  get fontSize() { return _settings.fontSize; },
  get wordWrap() { return _settings.wordWrap; },
  get showLineNumbers() { return _settings.showLineNumbers; },
  get tabSize() { return _settings.tabSize; },

  update(partial: Partial<Settings>) {
    _settings = { ..._settings, ...partial };
    saveSettings(_settings);
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
};
