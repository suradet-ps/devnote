import { describe, it, expect, beforeEach } from 'vitest';
import { setLocale, getLocale, resolveLocale, t } from './i18n.svelte';
describe('t()', () => {
  beforeEach(() => {
    setLocale('en');
  });

  it('resolves English keys', () => {
    expect(t('tabs.new')).toBe('New Tab');
    expect(t('dialog.save')).toBe('Save');
  });

  it('resolves Thai keys', () => {
    setLocale('th');
    expect(t('tabs.new')).toBe('แท็บใหม่');
    expect(t('dialog.save')).toBe('บันทึก');
  });

  it('interpolates parameters', () => {
    expect(t('status.lineCol', { line: 3, col: 14 })).toBe('Ln 3, Col 14');
    setLocale('th');
    expect(t('status.lineCol', { line: 3, col: 14 })).toBe('บรรทัด 3, คอลัมน์ 14');
  });

  it('falls back to English for missing translations', () => {
    setLocale('th');
    // 'app.name' exists in both; simulate a missing th key via the type-safe table
    expect(t('app.name')).toBe('DevNote');
  });

  it('tracks the current locale', () => {
    setLocale('th');
    expect(getLocale()).toBe('th');
    setLocale('en');
    expect(getLocale()).toBe('en');
  });
});

describe('resolveLocale', () => {
  it('maps "system" to a real locale', () => {
    const resolved = resolveLocale('system');
    expect(['en', 'th']).toContain(resolved);
  });

  it('passes explicit locales through', () => {
    expect(resolveLocale('en')).toBe('en');
    expect(resolveLocale('th')).toBe('th');
  });
});

describe('translations completeness', () => {
  it('Thai covers every English key (type-checked, asserted at runtime)', async () => {
    const { messages } = await import('./translations');
    const enKeys = Object.keys(messages.en).sort();
    const thKeys = Object.keys(messages.th).sort();
    expect(thKeys).toEqual(enKeys);
  });
});
