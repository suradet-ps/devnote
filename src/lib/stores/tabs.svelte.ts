import { detectLanguage } from '$lib/utils/detect-lang';

export interface Tab {
  id: string;
  path: string | null;
  fileName: string;
  content: string;
  savedContent: string;
  language: string;
  encoding: string;
  lineEnding: string;
  cursorLine: number;
  cursorCol: number;
  scrollTop: number;
}

/**
 * Frontend shape that matches the Rust `FilePayload` struct.
 * `path` is non-null in normal file payloads; recovery payloads may pass null.
 */
export interface FilePayload {
  path: string;
  content: string;
  file_name: string;
  encoding: string;
  line_ending: string;
}

let _tabs = $state<Tab[]>([]);
let _activeTabId = $state<string | null>(null);
let _untitledCounter = $state(0);

function extractFileName(path: string): string {
  return path.split('/').pop() || path.split('\\').pop() || path;
}

/** Internal: copy-on-write update of a single tab. Allocates exactly 1 array + 1 tab. */
function mutateTab(id: string, patch: (t: Tab) => Tab): void {
  const idx = _tabs.findIndex((t) => t.id === id);
  if (idx === -1) return;
  const next = _tabs.slice();
  next[idx] = patch(_tabs[idx]);
  _tabs = next;
}

function ensureOneTab(): void {
  if (_tabs.length > 0) return;
  _untitledCounter++;
  const tab: Tab = {
    id: crypto.randomUUID(),
    path: null,
    fileName: `untitled-${_untitledCounter}`,
    content: '',
    savedContent: '',
    language: 'text',
    encoding: 'UTF-8',
    lineEnding: 'LF',
    cursorLine: 1,
    cursorCol: 1,
    scrollTop: 0,
  };
  _tabs = [tab];
  _activeTabId = tab.id;
}

function setActiveAfterRemoval(removedId: string, removedIdx: number): void {
  if (_activeTabId !== removedId) return;
  _activeTabId =
    _tabs.length > 0 ? _tabs[Math.min(removedIdx, _tabs.length - 1)].id : null;
}

export const tabsStore = {
  get tabs() {
    return _tabs;
  },
  get activeTabId() {
    return _activeTabId;
  },

  get activeTab(): Tab | undefined {
    return _tabs.find((t) => t.id === _activeTabId);
  },

  newTab(): Tab {
    _untitledCounter++;
    const tab: Tab = {
      id: crypto.randomUUID(),
      path: null,
      fileName: `untitled-${_untitledCounter}`,
      content: '',
      savedContent: '',
      language: 'text',
      encoding: 'UTF-8',
      lineEnding: 'LF',
      cursorLine: 1,
      cursorCol: 1,
      scrollTop: 0,
    };
    _tabs = [..._tabs, tab];
    _activeTabId = tab.id;
    return tab;
  },

  openTab(payload: FilePayload): Tab {
    const existing = _tabs.find((t) => t.path === payload.path);
    if (existing) {
      _activeTabId = existing.id;
      return existing;
    }

    // Replace single empty untitled tab to avoid an unused tab after open
    if (
      _tabs.length === 1 &&
      _tabs[0].path === null &&
      _tabs[0].content === ''
    ) {
      _tabs = [];
    }

    const tab: Tab = {
      id: crypto.randomUUID(),
      path: payload.path,
      fileName: payload.file_name,
      content: payload.content,
      savedContent: payload.content,
      language: detectLanguage(payload.path),
      encoding: payload.encoding,
      lineEnding: payload.line_ending,
      cursorLine: 1,
      cursorCol: 1,
      scrollTop: 0,
    };
    _tabs = [..._tabs, tab];
    _activeTabId = tab.id;
    return tab;
  },

  closeTab(id: string): boolean {
    const idx = _tabs.findIndex((t) => t.id === id);
    if (idx === -1) return false;
    if (_tabs[idx].content !== _tabs[idx].savedContent) return false;

    _tabs = _tabs.filter((t) => t.id !== id);
    setActiveAfterRemoval(id, idx);
    ensureOneTab();
    return true;
  },

  forceCloseTab(id: string): void {
    const idx = _tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;
    _tabs = _tabs.filter((t) => t.id !== id);
    setActiveAfterRemoval(id, idx);
    ensureOneTab();
  },

  setActive(id: string): void {
    if (_tabs.some((t) => t.id === id)) {
      _activeTabId = id;
    }
  },

  updateContent(id: string, content: string): void {
    mutateTab(id, (t) => ({ ...t, content }));
  },

  markSaved(id: string, path: string): void {
    mutateTab(id, (t) => ({
      ...t,
      savedContent: t.content,
      path,
      fileName: extractFileName(path),
      language: detectLanguage(path),
    }));
  },

  renameTab(id: string, newName: string): void {
    mutateTab(id, (t) => ({ ...t, fileName: newName }));
  },

  updateCursor(id: string, line: number, col: number): void {
    mutateTab(id, (t) => ({ ...t, cursorLine: line, cursorCol: col }));
  },

  setLanguage(id: string, language: string): void {
    mutateTab(id, (t) => ({ ...t, language }));
  },

  reorder(fromIdx: number, toIdx: number): void {
    if (
      fromIdx < 0 ||
      toIdx < 0 ||
      fromIdx >= _tabs.length ||
      toIdx >= _tabs.length ||
      fromIdx === toIdx
    ) {
      return;
    }
    const next = _tabs.slice();
    const [moved] = next.splice(fromIdx, 1);
    next.splice(toIdx, 0, moved);
    _tabs = next;
  },

  getDirtyTabs(): Tab[] {
    return _tabs.filter((t) => t.content !== t.savedContent);
  },

  hasDirtyTabs(): boolean {
    return _tabs.some((t) => t.content !== t.savedContent);
  },

  /**
   * Test-only helper to reset store state. Not part of the public API.
   * @internal
   */
  __resetForTests(): void {
    _tabs = [];
    _activeTabId = null;
    _untitledCounter = 0;
  },
};
