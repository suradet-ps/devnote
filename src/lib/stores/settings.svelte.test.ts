// @vitest-environment happy-dom
import { describe, it, expect, beforeEach } from 'vitest';
import { settingsStore } from './settings.svelte';

// In the happy-dom test environment the Tauri store plugin is unavailable, so
// init() exercises the localStorage fallback path — which is exactly the
// migration surface we want to pin down.

describe('settingsStore migration & validation', () => {
  beforeEach(() => {
    localStorage.clear();
    settingsStore.__resetForTests();
  });

  it('migrates legacy sabot-settings with identical resolved values', async () => {
    localStorage.setItem(
      'sabot-settings',
      JSON.stringify({ theme: 'dark', fontSize: 16, wordWrap: true }),
    );
    await settingsStore.init();
    expect(settingsStore.settings.theme).toBe('dark');
    expect(settingsStore.settings.fontSize).toBe(16);
    expect(settingsStore.settings.wordWrap).toBe(true);
    // Unspecified keys resolve to defaults
    expect(settingsStore.settings.tabSize).toBe(4);
    expect(settingsStore.settings.insertSpaces).toBe(true);
    expect(settingsStore.settings.showLineNumbers).toBe(true);
  });

  it('prefers devnote-settings over the older sabot-settings key', async () => {
    localStorage.setItem('sabot-settings', JSON.stringify({ theme: 'dark' }));
    localStorage.setItem('devnote-settings', JSON.stringify({ theme: 'light', fontSize: 18 }));
    await settingsStore.init();
    expect(settingsStore.settings.theme).toBe('light');
    expect(settingsStore.settings.fontSize).toBe(18);
  });

  it('falls back to defaults for unknown keys and invalid values without throwing', async () => {
    localStorage.setItem(
      'devnote-settings',
      JSON.stringify({ fontSize: 99, tabSize: -3, theme: 'lime', bogus: true }),
    );
    await settingsStore.init();
    expect(settingsStore.settings.fontSize).toBe(14); // out of range → default
    expect(settingsStore.settings.tabSize).toBe(4); // out of range → default
    expect(settingsStore.settings.theme).toBe('system'); // unknown value → default
    expect('bogus' in settingsStore.settings).toBe(false); // unknown key dropped
  });

  it('returns defaults when no persisted settings exist', async () => {
    await settingsStore.init();
    expect(settingsStore.settings).toEqual({
      theme: 'system',
      fontSize: 14,
      fontFamily: 'JetBrains Mono',
      wordWrap: false,
      showLineNumbers: true,
      showStatusBar: true,
      tabSize: 4,
      insertSpaces: true,
    });
  });

  it('ignores malformed JSON without throwing', async () => {
    localStorage.setItem('devnote-settings', '{ not json');
    await settingsStore.init();
    expect(settingsStore.settings.fontSize).toBe(14);
  });

  it('update persists a sanitized snapshot back to localStorage', async () => {
    await settingsStore.init();
    settingsStore.update({ fontSize: 20, theme: 'dark' });
    // Give the fire-and-forget persist a tick to land
    await new Promise((r) => setTimeout(r, 10));
    const stored = JSON.parse(localStorage.getItem('devnote-settings') ?? '{}') as Record<string, unknown>;
    expect(stored.fontSize).toBe(20);
    expect(stored.theme).toBe('dark');
  });
});
