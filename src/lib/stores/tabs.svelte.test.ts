import { describe, it, expect, beforeEach } from 'vitest';

// We mock the dynamic import of detectLanguage; here we test the public surface.
import { tabsStore } from './tabs.svelte'; // resolves to tabs.svelte.ts

describe('tabsStore', () => {
  beforeEach(() => {
    tabsStore.__resetForTests();
  });

  it('starts empty and creates a fresh untitled tab on newTab', () => {
    expect(tabsStore.tabs.length).toBe(0);
    const t = tabsStore.newTab();
    expect(t.path).toBeNull();
    expect(t.fileName.startsWith('untitled-')).toBe(true);
    expect(t.content).toBe('');
    expect(t.savedContent).toBe('');
    expect(tabsStore.activeTabId).toBe(t.id);
  });

  it('forceCloseTab always keeps at least one tab', () => {
    const t = tabsStore.newTab();
    tabsStore.forceCloseTab(t.id);
    expect(tabsStore.tabs.length).toBe(1);
  });

  it('openTab focuses an existing tab with the same path', () => {
    const a = tabsStore.newTab();
    // Type some content so the "replace empty untitled" optimization doesn't fire
    tabsStore.updateContent(a.id, 'first tab content');
    const incoming = {
      path: '/a/b/file.txt',
      content: 'hello',
      file_name: 'file.txt',
      encoding: 'UTF-8',
      line_ending: 'LF',
    };
    tabsStore.openTab(incoming);
    expect(tabsStore.tabs.length).toBe(2);
    const beforeSecondOpen = tabsStore.tabs.map((t) => t.id);
    tabsStore.openTab(incoming); // open same path again
    expect(tabsStore.tabs.length).toBe(2);
    expect(tabsStore.tabs.map((t) => t.id)).toEqual(beforeSecondOpen);
    expect(tabsStore.activeTabId).toBe(tabsStore.tabs[1].id);
  });

  it('openTab replaces the single empty untitled tab', () => {
    tabsStore.newTab();
    expect(tabsStore.tabs.length).toBe(1);
    tabsStore.openTab({
      path: '/x.rs',
      content: 'fn main() {}',
      file_name: 'x.rs',
      encoding: 'UTF-8',
      line_ending: 'LF',
    });
    expect(tabsStore.tabs.length).toBe(1);
    expect(tabsStore.tabs[0].path).toBe('/x.rs');
  });

  it('updateContent marks the tab dirty', () => {
    const t = tabsStore.newTab();
    tabsStore.updateContent(t.id, 'typed');
    expect(tabsStore.hasDirtyTabs()).toBe(true);
    expect(tabsStore.getDirtyTabs().length).toBe(1);
  });

  it('markSaved clears dirty state', () => {
    const t = tabsStore.newTab();
    tabsStore.updateContent(t.id, 'typed');
    expect(tabsStore.hasDirtyTabs()).toBe(true);
    tabsStore.markSaved(t.id, '/x.txt');
    expect(tabsStore.hasDirtyTabs()).toBe(false);
    expect(tabsStore.tabs[0].path).toBe('/x.txt');
  });

  it('closeTab refuses to close a dirty tab', () => {
    const t = tabsStore.newTab();
    tabsStore.updateContent(t.id, 'typed');
    expect(tabsStore.closeTab(t.id)).toBe(false);
    expect(tabsStore.tabs.length).toBe(1);
  });

  it('closeTab closes a clean tab', () => {
    const t = tabsStore.newTab();
    expect(tabsStore.closeTab(t.id)).toBe(true);
    // It will create a new untitled tab to keep count at 1
    expect(tabsStore.tabs.length).toBe(1);
  });

  it('setActive switches the active tab', () => {
    const a = tabsStore.newTab();
    const b = tabsStore.newTab();
    expect(tabsStore.activeTabId).toBe(b.id);
    tabsStore.setActive(a.id);
    expect(tabsStore.activeTabId).toBe(a.id);
  });

  it('setActive ignores unknown ids', () => {
    const a = tabsStore.newTab();
    tabsStore.setActive('nonexistent');
    expect(tabsStore.activeTabId).toBe(a.id);
  });

  it('reorder moves a tab', () => {
    const a = tabsStore.newTab();
    const b = tabsStore.newTab();
    const c = tabsStore.newTab();
    expect(tabsStore.tabs.map((t) => t.id)).toEqual([a.id, b.id, c.id]);
    tabsStore.reorder(0, 2);
    expect(tabsStore.tabs.map((t) => t.id)).toEqual([b.id, c.id, a.id]);
  });

  it('reorder is a no-op for out-of-bounds indices', () => {
    const a = tabsStore.newTab();
    const b = tabsStore.newTab();
    const before = tabsStore.tabs.map((t) => t.id);
    tabsStore.reorder(-1, 0);
    tabsStore.reorder(0, 5);
    tabsStore.reorder(0, 0);
    expect(tabsStore.tabs.map((t) => t.id)).toEqual(before);
  });
});
