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

  it('untitled counter produces unique file names across closes', () => {
    const a = tabsStore.newTab();
    const b = tabsStore.newTab();
    const c = tabsStore.newTab();
    const names = tabsStore.tabs.map((t) => t.fileName);
    expect(new Set(names).size).toBe(3);
    // ensureOneTab() only fires when the last tab is closed — and it must
    // consume a fresh counter value so names never collide with history
    tabsStore.forceCloseTab(a.id);
    tabsStore.forceCloseTab(b.id);
    tabsStore.forceCloseTab(c.id);
    expect(tabsStore.tabs.length).toBe(1);
    expect(tabsStore.tabs[0].fileName).toBe('untitled-4');
  });

  it('markSaved updates fileName, path and language from the new path', () => {
    const t = tabsStore.newTab();
    tabsStore.updateContent(t.id, 'fn main() {}');
    tabsStore.markSaved(t.id, '/a/b/main.rs');
    const saved = tabsStore.tabs[0];
    expect(saved.fileName).toBe('main.rs');
    expect(saved.path).toBe('/a/b/main.rs');
    expect(saved.language).toBe('rust');
    expect(saved.content).toBe('fn main() {}');
  });

  it('forceCloseTab removes a dirty tab and keeps at least one tab', () => {
    const t = tabsStore.newTab();
    tabsStore.updateContent(t.id, 'typed');
    expect(tabsStore.hasDirtyTabs()).toBe(true);
    tabsStore.forceCloseTab(t.id);
    expect(tabsStore.tabs.length).toBe(1);
    expect(tabsStore.tabs[0].id).not.toBe(t.id);
  });

  it('closing the active middle tab activates the next neighbor', () => {
    const a = tabsStore.newTab();
    const b = tabsStore.newTab();
    const c = tabsStore.newTab();
    tabsStore.setActive(b.id);
    expect(tabsStore.closeTab(b.id)).toBe(true);
    expect(tabsStore.activeTabId).toBe(c.id);
  });

  it('closing the active last tab activates the previous neighbor', () => {
    const a = tabsStore.newTab();
    const b = tabsStore.newTab();
    const c = tabsStore.newTab();
    tabsStore.setActive(c.id);
    tabsStore.closeTab(c.id);
    expect(tabsStore.activeTabId).toBe(b.id);
  });

  it('closing an inactive tab does not move the active tab', () => {
    const a = tabsStore.newTab();
    const b = tabsStore.newTab();
    tabsStore.setActive(a.id);
    tabsStore.closeTab(b.id);
    expect(tabsStore.activeTabId).toBe(a.id);
  });
});
