export interface Tab {
  id: string;
  path: string | null;
  fileName: string;
  content: string;
  savedContent: string;
  language: string;
  cursorLine: number;
  cursorCol: number;
  scrollTop: number;
}

export interface FilePayload {
  path: string;
  content: string;
  file_name: string;
}

let _tabs = $state<Tab[]>([]);
let _activeTabId = $state<string | null>(null);
let _untitledCounter = $state(0);

export const tabsStore = {
  get tabs() { return _tabs; },
  get activeTabId() { return _activeTabId; },

  get activeTab(): Tab | undefined {
    return _tabs.find(t => t.id === _activeTabId);
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
      cursorLine: 1,
      cursorCol: 1,
      scrollTop: 0,
    };
    _tabs = [..._tabs, tab];
    _activeTabId = tab.id;
    return tab;
  },

  openTab(payload: FilePayload): Tab {
    const existing = _tabs.find(t => t.path === payload.path);
    if (existing) {
      _activeTabId = existing.id;
      return existing;
    }
    const tab: Tab = {
      id: crypto.randomUUID(),
      path: payload.path,
      fileName: payload.file_name,
      content: payload.content,
      savedContent: payload.content,
      language: detectLangFromPath(payload.path),
      cursorLine: 1,
      cursorCol: 1,
      scrollTop: 0,
    };
    _tabs = [..._tabs, tab];
    _activeTabId = tab.id;
    return tab;
  },

  closeTab(id: string): boolean {
    const idx = _tabs.findIndex(t => t.id === id);
    if (idx === -1) return false;
    const tab = _tabs[idx];
    if (tab.content !== tab.savedContent) return false;

    _tabs = _tabs.filter(t => t.id !== id);
    if (_activeTabId === id) {
      _activeTabId = _tabs.length > 0 ? _tabs[Math.min(idx, _tabs.length - 1)].id : null;
    }
    return true;
  },

  forceCloseTab(id: string) {
    const idx = _tabs.findIndex(t => t.id === id);
    if (idx === -1) return;
    _tabs = _tabs.filter(t => t.id !== id);
    if (_activeTabId === id) {
      _activeTabId = _tabs.length > 0 ? _tabs[Math.min(idx, _tabs.length - 1)].id : null;
    }
  },

  setActive(id: string) {
    _activeTabId = id;
  },

  updateContent(id: string, content: string) {
    _tabs = _tabs.map(t => t.id === id ? { ...t, content } : t);
  },

  markSaved(id: string, path: string) {
    const tab = _tabs.find(t => t.id === id);
    if (tab) {
      _tabs = _tabs.map(t => t.id === id ? {
        ...t,
        savedContent: t.content,
        path,
        fileName: path.split('/').pop() || path.split('\\').pop() || t.fileName,
        language: detectLangFromPath(path),
      } : t);
    }
  },

  updateCursor(id: string, line: number, col: number) {
    _tabs = _tabs.map(t => t.id === id ? { ...t, cursorLine: line, cursorCol: col } : t);
  },

  updateScrollTop(id: string, scrollTop: number) {
    _tabs = _tabs.map(t => t.id === id ? { ...t, scrollTop } : t);
  },

  getDirtyTabs(): Tab[] {
    return _tabs.filter(t => t.content !== t.savedContent);
  },

  hasDirtyTabs(): boolean {
    return _tabs.some(t => t.content !== t.savedContent);
  },

  closeAll() {
    _tabs = [];
    _activeTabId = null;
  }
};

function detectLangFromPath(path: string | null): string {
  if (!path) return 'text';
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  const extMap: Record<string, string> = {
    rs: 'rust', ts: 'typescript', tsx: 'typescript',
    js: 'javascript', jsx: 'javascript',
    py: 'python', html: 'html', css: 'css',
    md: 'markdown', json: 'json', sql: 'sql',
    xml: 'xml', vue: 'vue', cpp: 'cpp', cc: 'cpp',
    c: 'cpp', h: 'cpp', java: 'java', php: 'php',
    toml: 'toml', yaml: 'yaml', yml: 'yaml',
  };
  return extMap[ext] ?? 'text';
}
