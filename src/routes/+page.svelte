<script lang="ts">
  import { onMount } from 'svelte';
  import TabBar from '$lib/components/TabBar.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import FindReplace from '$lib/components/FindReplace.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { tabsStore } from '$lib/stores/tabs.svelte';
  import { recentStore } from '$lib/stores/recent.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { FilePayload } from '$lib/stores/tabs.svelte';

  let appWindow: ReturnType<typeof getCurrentWindow> | null = null;
  function getAppWindow() {
    if (!appWindow) appWindow = getCurrentWindow();
    return appWindow;
  }

  let showFindReplace = $state(false);
  let showGoToLine = $state(false);
  let goToLineValue = $state('');
  let showRecentDialog = $state(false);
  let toastMessage = $state('');
  let toastVisible = $state(false);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  let confirmOpen = $state(false);
  let confirmTitle = $state('');
  let confirmMessage = $state('');
  let confirmShowSave = $state(true);
  let confirmShowDiscard = $state(true);
  let confirmShowCancel = $state(true);
  let confirmSaveLabel = $state('Save');
  let confirmResolve: ((value: string) => void) | null = null;

  function showConfirmDialog(title: string, message: string, opts?: { saveLabel?: string; showSave?: boolean; showDiscard?: boolean; showCancel?: boolean }): Promise<string> {
    return new Promise((resolve) => {
      confirmTitle = title;
      confirmMessage = message;
      confirmShowSave = opts?.showSave ?? true;
      confirmShowDiscard = opts?.showDiscard ?? true;
      confirmShowCancel = true;
      confirmSaveLabel = opts?.saveLabel ?? 'Save';
      confirmOpen = true;
      confirmResolve = resolve;
    });
  }

  function handleConfirmSave() {
    confirmOpen = false;
    confirmResolve?.('save');
  }

  function handleConfirmDiscard() {
    confirmOpen = false;
    confirmResolve?.('discard');
  }

  function handleConfirmCancel() {
    confirmOpen = false;
    confirmResolve?.('cancel');
  }

  function showToast(message: string) {
    toastMessage = message;
    toastVisible = true;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { toastVisible = false; }, 4000);
  }

  async function handleOpenFile() {
    try {
      const payload = await invoke<FilePayload | null>('open_file');
      if (payload) {
        tabsStore.openTab(payload);
        await recentStore.add(payload.path);
      }
    } catch (e: any) {
      showToast(`Failed to open file: ${e}`);
    }
  }

  async function handleOpenRecent(path: string) {
    try {
      const payload = await invoke<FilePayload>('read_file', { path });
      tabsStore.openTab(payload);
      await recentStore.add(path);
    } catch (e: any) {
      const err = String(e);
      if (err.includes('not found') || err.includes('No such file')) {
        await recentStore.remove(path);
        showToast(`File not found: ${path}`);
      } else {
        showToast(`Failed to open: ${err}`);
      }
    }
    showRecentDialog = false;
  }

  async function handleOpenFromPath(path: string) {
    try {
      const size = await invoke<number>('check_file_size', { path });
      if (size > 10 * 1024 * 1024) {
        const result = await showConfirmDialog(
          'Large File',
          `This file is ${(size / (1024 * 1024)).toFixed(1)} MB. Opening large files may be slow. Continue?`,
        );
        if (result !== 'save') return;
      }
      const payload = await invoke<FilePayload>('read_file_with_encoding', { path });
      tabsStore.openTab(payload);
      await recentStore.add(path);
    } catch (e: any) {
      showToast(`Failed to open: ${e}`);
    }
  }

  async function handleSave() {
    const tab = tabsStore.activeTab;
    if (!tab) return;

    if (!tab.path) {
      await handleSaveAs();
      return;
    }

    try {
      await invoke('save_file', { path: tab.path, content: tab.content });
      tabsStore.markSaved(tab.id, tab.path);
      updateWindowTitle();
    } catch (e: any) {
      const err = String(e);
      if (err.includes('Permission denied') || err.includes('read-only')) {
        const result = await showConfirmDialog(
          'Cannot Save',
          `Cannot save "${tab.fileName}" — the file is read-only. Save a copy instead?`,
          { showDiscard: false, saveLabel: 'Save Copy' },
        );
        if (result === 'save') {
          await handleSaveAs();
        }
      } else {
        showToast(`Failed to save: ${err}`);
      }
    }
  }

  async function handleSaveAs() {
    const tab = tabsStore.activeTab;
    if (!tab) return;

    try {
      const newPath = await invoke<string | null>('save_file_as', {
        content: tab.content,
        suggestedName: tab.fileName,
      });
      if (newPath) {
        tabsStore.markSaved(tab.id, newPath);
        await recentStore.add(newPath);
        updateWindowTitle();
      }
    } catch (e: any) {
      showToast(`Failed to save as: ${e}`);
    }
  }

  function handleContentChange(content: string) {
    const tab = tabsStore.activeTab;
    if (tab) {
      tabsStore.updateContent(tab.id, content);
    }
  }

  function handleCursorUpdate(line: number, col: number) {
    const tab = tabsStore.activeTab;
    if (tab) {
      tabsStore.updateCursor(tab.id, line, col);
    }
  }

  function updateWindowTitle() {
    const tab = tabsStore.activeTab;
    if (!tab) {
      invoke('set_window_title', { title: 'text-rs' }).catch(() => {});
      return;
    }
    const dirty = tab.content !== tab.savedContent ? '\u2022 ' : '';
    const title = `${dirty}${tab.fileName} \u2014 text-rs`;
    invoke('set_window_title', { title }).catch(() => {});
  }

  // Update title when tab/file changes
  $effect(() => {
    const tab = tabsStore.activeTab;
    if (tab) {
      updateWindowTitle();
    }
    // Track content changes
    void tab?.content;
  });

  async function handleTabCloseRequest(e: CustomEvent) {
    const tabId = e.detail.tabId;
    const tab = tabsStore.tabs.find(t => t.id === tabId);
    if (!tab) return;

    const result = await showConfirmDialog(
      'Save changes?',
      `"${tab.fileName}" has unsaved changes.`,
    );

    if (result === 'save') {
      if (tab.path) {
        try {
          await invoke('save_file', { path: tab.path, content: tab.content });
          tabsStore.markSaved(tab.id, tab.path);
          tabsStore.forceCloseTab(tabId);
        } catch (e: any) {
          showToast(`Failed to save: ${e}`);
        }
      } else {
        await handleSaveAs();
        tabsStore.forceCloseTab(tabId);
      }
    } else if (result === 'discard') {
      tabsStore.forceCloseTab(tabId);
    }
  }

  async function handleCloseRequest() {
    const dirtyTabs = tabsStore.getDirtyTabs();
    if (dirtyTabs.length === 0) {
      getAppWindow().close();
      return;
    }

    const result = await showConfirmDialog(
      'Unsaved Changes',
      `You have ${dirtyTabs.length} unsaved file(s). Save before closing?`,
      { saveLabel: 'Save All' },
    );

    if (result === 'save') {
      for (const tab of dirtyTabs) {
        if (tab.path) {
          try {
            await invoke('save_file', { path: tab.path, content: tab.content });
            tabsStore.markSaved(tab.id, tab.path);
          } catch (e) {
            console.error('Failed to save:', e);
          }
        }
      }
      getAppWindow().close();
    } else if (result === 'discard') {
      getAppWindow().close();
    }
  }

  function handleGoToLine() {
    const line = parseInt(goToLineValue, 10);
    if (isNaN(line) || line < 1) return;

    window.dispatchEvent(new CustomEvent('editor-action', {
      detail: { action: 'go-to-line', line },
    }));
    showGoToLine = false;
    goToLineValue = '';
  }

  $effect(() => {
    if (showGoToLine) {
      setTimeout(() => {
        document.querySelector<HTMLInputElement>('.goto-line-input')?.focus();
      }, 50);
    }
  });

  function handleGlobalKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;

    if (mod && e.key === 'n') {
      e.preventDefault();
      tabsStore.newTab();
    } else if (mod && e.key === 'o') {
      e.preventDefault();
      handleOpenFile();
    } else if (mod && !e.shiftKey && e.key === 's') {
      e.preventDefault();
      handleSave();
    } else if (mod && e.shiftKey && e.key === 's') {
      e.preventDefault();
      handleSaveAs();
    } else if (mod && e.key === 'w') {
      e.preventDefault();
      const tab = tabsStore.activeTab;
      if (tab) {
        if (tab.content !== tab.savedContent) {
          handleTabCloseRequest({ detail: { tabId: tab.id } } as CustomEvent);
        } else {
          tabsStore.forceCloseTab(tab.id);
        }
      }
    } else if (mod && e.key === 'f') {
      e.preventDefault();
      showFindReplace = !showFindReplace;
    } else if (mod && e.key === 'h') {
      e.preventDefault();
      showFindReplace = true;
    } else if (mod && e.key === 'g') {
      e.preventDefault();
      showGoToLine = true;
    } else if (mod && e.key === '=') {
      e.preventDefault();
      settingsStore.increaseFontSize();
    } else if (mod && e.key === '-') {
      e.preventDefault();
      settingsStore.decreaseFontSize();
    } else if (mod && e.key === '0') {
      e.preventDefault();
      settingsStore.resetFontSize();
    } else if (e.altKey && e.key === 'z') {
      e.preventDefault();
      settingsStore.toggleWordWrap();
    } else if (e.key === 'F3') {
      e.preventDefault();
      window.dispatchEvent(new CustomEvent('editor-action', {
        detail: { action: 'search-next' },
      }));
    } else if (e.shiftKey && e.key === 'F3') {
      e.preventDefault();
      window.dispatchEvent(new CustomEvent('editor-action', {
        detail: { action: 'search-prev' },
      }));
    } else if (mod && !e.shiftKey && e.key === 'Tab') {
      e.preventDefault();
      const tabIds = tabsStore.tabs.map(t => t.id);
      const currentIdx = tabIds.indexOf(tabsStore.activeTabId ?? '');
      const nextIdx = currentIdx >= tabIds.length - 1 ? 0 : currentIdx + 1;
      tabsStore.setActive(tabIds[nextIdx]);
    } else if (mod && e.shiftKey && e.key === 'Tab') {
      e.preventDefault();
      const tabIds = tabsStore.tabs.map(t => t.id);
      const currentIdx = tabIds.indexOf(tabsStore.activeTabId ?? '');
      const prevIdx = currentIdx <= 0 ? tabIds.length - 1 : currentIdx - 1;
      tabsStore.setActive(tabIds[prevIdx]);
    } else if (mod && e.key >= '1' && e.key <= '9') {
      e.preventDefault();
      const idx = parseInt(e.key) - 1;
      if (idx < tabsStore.tabs.length) {
        tabsStore.setActive(tabsStore.tabs[idx].id);
      }
    }
  }

  // Handle file drop
  async function handleFileDrop(e: DragEvent) {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (!files) return;

    for (const file of Array.from(files)) {
      try {
        // In Tauri, File objects have a 'path' property
        const filePath = (file as any).path ?? file.name;
        await handleOpenFromPath(filePath);
      } catch (err) {
        console.error('Failed to open dropped file:', err);
      }
    }
  }

  // Auto-save recovery data every 30 seconds
  let recoveryInterval: ReturnType<typeof setInterval> | null = null;

  async function saveRecovery() {
    try {
      const tabs = tabsStore.tabs.map(t => ({
        file_name: t.fileName,
        content: t.content,
        path: t.path,
        saved_at: new Date().toISOString(),
      }));
      await invoke('save_recovery_data', { tabs });
    } catch {
      // Silent - recovery save is best-effort
    }
  }

  async function checkRecovery() {
    try {
      const entries = await invoke<Array<{ file_name: string; content: string; path: string | null }> | null>('check_recovery_data');
      if (entries && entries.length > 0) {
        const names = entries.map(e => e.file_name).join(', ');
        const result = await showConfirmDialog(
          'Session Recovery',
          `Found unsaved files from previous session: ${names}. Restore them?`,
          { showDiscard: true, showCancel: true, saveLabel: 'Restore' },
        );
        if (result === 'save') {
          for (const entry of entries) {
            tabsStore.openTab({
              path: entry.path ?? '',
              content: entry.content,
              file_name: entry.file_name,
            });
          }
        }
        await invoke('clear_recovery_data');
      }
    } catch {
      // Silent - recovery check is best-effort
    }
  }

  onMount(() => {
    const init = async () => {
      await settingsStore.init();
      await recentStore.refresh();

      if (tabsStore.tabs.length === 0) {
        tabsStore.newTab();
      }

      // Check for recovery data on startup
      await checkRecovery();

      updateWindowTitle();
    };
    init();

    // Start recovery auto-save interval
    recoveryInterval = setInterval(() => saveRecovery(), 30000);

    window.addEventListener('keydown', handleGlobalKeydown);
    window.addEventListener('tab-close-request', handleTabCloseRequest as unknown as EventListener);
    window.addEventListener('window-close-request', handleCloseRequest);
    window.addEventListener('menu-new-tab', () => tabsStore.newTab());
    window.addEventListener('menu-open-file', () => handleOpenFile());
    window.addEventListener('menu-save', () => handleSave());
    window.addEventListener('menu-save-as', () => handleSaveAs());
    window.addEventListener('menu-close-tab', () => {
      const tab = tabsStore.activeTab;
      if (tab) {
        if (tab.content !== tab.savedContent) {
          handleTabCloseRequest({ detail: { tabId: tab.id } } as CustomEvent);
        } else {
          tabsStore.forceCloseTab(tab.id);
        }
      }
    });
    window.addEventListener('menu-undo', () => {
      window.dispatchEvent(new CustomEvent('editor-action', { detail: { action: 'undo' } }));
    });
    window.addEventListener('menu-redo', () => {
      window.dispatchEvent(new CustomEvent('editor-action', { detail: { action: 'redo' } }));
    });
    window.addEventListener('menu-cut', () => {
      window.dispatchEvent(new CustomEvent('editor-action', { detail: { action: 'cut' } }));
    });
    window.addEventListener('menu-copy', () => {
      window.dispatchEvent(new CustomEvent('editor-action', { detail: { action: 'copy' } }));
    });
    window.addEventListener('menu-paste', () => {
      window.dispatchEvent(new CustomEvent('editor-action', { detail: { action: 'paste' } }));
    });
    window.addEventListener('menu-select-all', () => {
      window.dispatchEvent(new CustomEvent('editor-action', { detail: { action: 'select-all' } }));
    });
    window.addEventListener('menu-find', () => { showFindReplace = true; });
    window.addEventListener('menu-find-replace', () => { showFindReplace = true; });
    window.addEventListener('menu-go-to-line', () => { showGoToLine = true; });
    window.addEventListener('menu-zoom-in', () => settingsStore.increaseFontSize());
    window.addEventListener('menu-zoom-out', () => settingsStore.decreaseFontSize());
    window.addEventListener('menu-zoom-reset', () => settingsStore.resetFontSize());
    window.addEventListener('menu-word-wrap', () => settingsStore.toggleWordWrap());
    window.addEventListener('menu-status-bar', () => settingsStore.toggleStatusBar());
    window.addEventListener('menu-open-recent', (e) => {
      const path = (e as CustomEvent<string>).detail;
      handleOpenRecent(path);
    });
    window.addEventListener('menu-about', () => {
      showConfirmDialog(
        'About text-rs',
        'text-rs v0.2.0\nA fast, lightweight text editor.\nBuilt with Tauri, Svelte 5, and CodeMirror 6.',
        { showDiscard: false, showCancel: false, saveLabel: 'OK' },
      ).then(() => {});
    });

    // File opened from OS (double-click or argv)
    const unlistenFileOpened = getAppWindow().listen<string[]>('file-opened', async (event) => {
      for (const filePath of event.payload) {
        await handleOpenFromPath(filePath);
      }
    });

    // File drop on window
    const unlistenDragDrop = getAppWindow().onDragDropEvent(async (event) => {
      if (event.payload.type === 'drop') {
        for (const filePath of event.payload.paths) {
          await handleOpenFromPath(filePath);
        }
      }
    });

    return () => {
      if (recoveryInterval) clearInterval(recoveryInterval);
      window.removeEventListener('keydown', handleGlobalKeydown);
      window.removeEventListener('tab-close-request', handleTabCloseRequest as unknown as EventListener);
      window.removeEventListener('window-close-request', handleCloseRequest);
      unlistenFileOpened.then(fn => fn());
      unlistenDragDrop.then(fn => fn());
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="app" ondragover={(e) => e.preventDefault()} ondrop={handleFileDrop}>
  <TabBar />
  <div class="editor-area">
    {#if tabsStore.activeTab}
      <FindReplace
        show={showFindReplace}
        onClose={() => showFindReplace = false}
      />
      {#if showGoToLine}
        <div class="goto-line-panel" role="dialog" aria-label="Go to Line">
          <input
            class="goto-line-input"
            type="number"
            min="1"
            placeholder="Line number"
            bind:value={goToLineValue}
            onkeydown={(e) => {
              if (e.key === 'Enter') handleGoToLine();
              if (e.key === 'Escape') { showGoToLine = false; goToLineValue = ''; }
            }}
          />
          <button class="goto-line-btn" onclick={handleGoToLine}>Go</button>
        </div>
      {/if}
      <Editor
        tabId={tabsStore.activeTab.id}
        content={tabsStore.activeTab.content}
        language={tabsStore.activeTab.language}
        onContentChange={handleContentChange}
        onCursorUpdate={handleCursorUpdate}
      />
    {:else}
      <div class="empty-state">
        <p>No open files</p>
        <p class="empty-hint">Ctrl+N to create a new tab, Ctrl+O to open a file</p>
      </div>
    {/if}
  </div>
  <StatusBar />

  {#if showRecentDialog}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="toast-backdrop" onclick={() => showRecentDialog = false} onkeydown={() => {}}>
      <div class="recent-dialog" onclick={(e) => e.stopPropagation()} onkeydown={() => {}} role="dialog" tabindex="-1">
        <h3>Open Recent</h3>
        {#if recentStore.recentFiles.length === 0}
          <p class="recent-empty">No recent files</p>
        {:else}
          <div class="recent-list">
            {#each recentStore.recentFiles as path}
              <button class="recent-item" onclick={() => handleOpenRecent(path)}>
                {path}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if toastVisible}
    <div class="toast" role="alert">{toastMessage}</div>
  {/if}
</div>

<ConfirmDialog
  open={confirmOpen}
  title={confirmTitle}
  message={confirmMessage}
  showSave={confirmShowSave}
  showDiscard={confirmShowDiscard}
  showCancel={confirmShowCancel}
  saveLabel={confirmSaveLabel}
  onSave={handleConfirmSave}
  onDiscard={handleConfirmDiscard}
  onCancel={handleConfirmCancel}
/>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--canvas);
    overflow: hidden;
  }

  .editor-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    background: var(--canvas);
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    gap: var(--sp-xs);
  }

  .empty-hint {
    font-size: 12px;
    color: var(--muted-soft);
  }

  .goto-line-panel {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--surface-card);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    padding: var(--sp-md);
    display: flex;
    gap: var(--sp-xs);
    z-index: 60;
    box-shadow: 0 8px 32px rgba(20, 20, 19, 0.15);
  }

  .goto-line-input {
    width: 120px;
    height: 32px;
    padding: 0 var(--sp-xs);
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    font-size: 14px;
    color: var(--ink);
    text-align: center;
  }

  .goto-line-input:focus {
    border-color: var(--primary);
    outline: none;
  }

  .goto-line-btn {
    height: 32px;
    padding: 0 var(--sp-md);
    background: var(--primary);
    color: var(--on-primary);
    border-radius: var(--r-md);
    font-size: 13px;
    font-weight: 500;
  }

  .goto-line-btn:hover {
    background: var(--primary-active);
  }

  .toast {
    position: fixed;
    bottom: 48px;
    right: var(--sp-md);
    background: var(--surface-dark);
    color: var(--on-dark);
    border-left: 4px solid var(--error);
    border-radius: var(--r-md);
    padding: var(--sp-sm) var(--sp-md);
    font-size: 13px;
    z-index: 550;
    box-shadow: 0 4px 16px rgba(20, 20, 19, 0.25);
    animation: toastIn 0.2s ease-out;
    max-width: 400px;
  }

  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }

  @keyframes toastIn {
    from { transform: translateY(8px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .toast-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(20, 20, 19, 0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 500;
  }

  .recent-dialog {
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    padding: var(--sp-lg);
    min-width: 400px;
    max-width: 500px;
    max-height: 400px;
    overflow-y: auto;
    box-shadow: 0 8px 32px rgba(20, 20, 19, 0.2);
  }

  .recent-dialog h3 {
    font-size: 15px;
    font-weight: 500;
    color: var(--ink);
    margin-bottom: var(--sp-md);
  }

  .recent-empty {
    font-size: 13px;
    color: var(--muted-soft);
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .recent-item {
    display: block;
    width: 100%;
    padding: var(--sp-xs) var(--sp-sm);
    font-size: 13px;
    color: var(--body);
    text-align: left;
    border-radius: var(--r-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background 0.1s;
  }

  .recent-item:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }
</style>
