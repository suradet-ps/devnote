<script lang="ts">
  import { onMount, tick } from 'svelte';
  import TabBar from '$lib/components/TabBar.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import FindReplace from '$lib/components/FindReplace.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { tabsStore, type Tab } from '$lib/stores/tabs.svelte';
  import { recentStore } from '$lib/stores/recent.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { ipc } from '$lib/tauri/ipc';
  import { dispatchEditorAction } from '$lib/editor/actions';
  import { errorMessage } from '$lib/utils/error';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen as listenTauriEvent, type UnlistenFn } from '@tauri-apps/api/event';
  import type { FilePayload, RecoveryEntry } from '$lib/types/ipc';

  const SOFT_LIMIT_BYTES = 10 * 1024 * 1024;
  const RECOVERY_INTERVAL_MS = 15_000;
  const TOAST_DURATION_MS = 4_000;
  const SOFT_LIMIT_MB = SOFT_LIMIT_BYTES / (1024 * 1024);

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

  // The active tab is read by the template (which passes props to <Editor>).
  // We expose it through a $derived so the Editor always sees consistent,
  // freshly-evaluated values when props are bound to its fields.
  const activeTab = $derived(tabsStore.activeTab);

  // Confirm dialog: queue-based to avoid losing concurrent resolvers.
  // A single shared `confirmResolve` would orphan any in-flight promise
  // if a second dialog was opened; the queue keeps each resolver paired
  // with its own dialog lifecycle.
  type ConfirmResult = 'save' | 'discard' | 'cancel';
  interface ConfirmRequest {
    title: string;
    message: string;
    showSave: boolean;
    showDiscard: boolean;
    showCancel: boolean;
    saveLabel: string;
    resolve: (r: ConfirmResult) => void;
  }
  let confirmOpen = $state(false);
  let confirmTitle = $state('');
  let confirmMessage = $state('');
  let confirmShowSave = $state(true);
  let confirmShowDiscard = $state(true);
  let confirmShowCancel = $state(true);
  let confirmSaveLabel = $state('Save');
  let confirmQueue: ConfirmRequest[] = $state([]);

  function showConfirmDialog(
    title: string,
    message: string,
    opts?: { saveLabel?: string; showSave?: boolean; showDiscard?: boolean; showCancel?: boolean }
  ): Promise<ConfirmResult> {
    return new Promise<ConfirmResult>((resolve) => {
      const req: ConfirmRequest = {
        title,
        message,
        showSave: opts?.showSave ?? true,
        showDiscard: opts?.showDiscard ?? true,
        showCancel: opts?.showCancel ?? true,
        saveLabel: opts?.saveLabel ?? 'Save',
        resolve,
      };
      // If a dialog is already showing, queue this one. The first
      // dialog's user action will pop the queue and show the next.
      if (confirmOpen) {
        confirmQueue = [...confirmQueue, req];
        return;
      }
      presentRequest(req);
    });
  }

  function presentRequest(req: ConfirmRequest) {
    confirmTitle = req.title;
    confirmMessage = req.message;
    confirmShowSave = req.showSave;
    confirmShowDiscard = req.showDiscard;
    confirmShowCancel = req.showCancel;
    confirmSaveLabel = req.saveLabel;
    confirmOpen = true;
    // Defer attaching the resolver to the NEXT microtask so the
    // dialog's buttons are mounted (and their onclick handlers wired)
    // before the user can click them. Without this the very first
    // click on a freshly-shown dialog can race the prop binding.
    queueMicrotask(() => {
      pendingResolve = req.resolve;
    });
  }

  let pendingResolve: ((r: ConfirmResult) => void) | null = null;

  function resolveConfirm(result: ConfirmResult) {
    confirmOpen = false;
    const resolve = pendingResolve;
    pendingResolve = null;
    resolve?.(result);
    // If more dialogs were queued, show the next one on the next tick.
    if (confirmQueue.length > 0) {
      const [next, ...rest] = confirmQueue;
      confirmQueue = rest;
      queueMicrotask(() => presentRequest(next));
    }
  }

  function handleConfirmSave() {
    resolveConfirm('save');
  }

  function handleConfirmDiscard() {
    resolveConfirm('discard');
  }

  function handleConfirmCancel() {
    resolveConfirm('cancel');
  }

  function showToast(message: string) {
    toastMessage = message;
    toastVisible = true;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastVisible = false;
    }, TOAST_DURATION_MS);
  }

  async function handleOpenFile() {
    try {
      const payload = await ipc.openFile();
      if (payload) {
        tabsStore.openTab(payload);
        await recentStore.add(payload.path);
      }
    } catch (e: unknown) {
      showToast(`Failed to open file: ${errorMessage(e)}`);
    }
  }

  async function handleOpenRecent(path: string) {
    try {
      const payload = await ipc.readFile(path);
      tabsStore.openTab(payload);
      await recentStore.add(path);
    } catch (e: unknown) {
      const err = errorMessage(e);
      if (err.toLowerCase().includes('not found') || err.toLowerCase().includes('no such file')) {
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
      const sizeStr = await ipc.checkFileSize(path);
      const size = Number(sizeStr);
      if (size > SOFT_LIMIT_BYTES) {
        const result = await showConfirmDialog(
          'Large File',
          `This file is ${(size / (1024 * 1024)).toFixed(1)} MB. Opening large files may be slow. Continue?`,
        );
        if (result !== 'save') return;
      }
      const payload = await ipc.readFile(path);
      tabsStore.openTab(payload);
      await recentStore.add(path);
    } catch (e: unknown) {
      showToast(`Failed to open: ${errorMessage(e)}`);
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
      await ipc.saveFile({
        path: tab.path,
        content: tab.content,
        lineEnding: tab.lineEnding,
        encoding: tab.encoding,
      });
      tabsStore.markSaved(tab.id, tab.path);
      updateWindowTitle();
    } catch (e: unknown) {
      const err = errorMessage(e);
      if (err.toLowerCase().includes('permission denied') || err.toLowerCase().includes('read-only')) {
        const result = await showConfirmDialog(
          'Cannot Save',
          `Cannot save "${tab.fileName}" \u2014 the file is read-only. Save a copy instead?`,
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
      const newPath = await ipc.saveFileAs({
        content: tab.content,
        suggestedName: tab.fileName,
        lineEnding: tab.lineEnding,
        encoding: tab.encoding,
      });
      if (newPath) {
        tabsStore.markSaved(tab.id, newPath);
        await recentStore.add(newPath);
        updateWindowTitle();
      }
    } catch (e: unknown) {
      showToast(`Failed to save as: ${errorMessage(e)}`);
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
      ipc.setWindowTitle('Text RS').catch(() => {});
      return;
    }
    const dirty = tab.content !== tab.savedContent ? '\u2022 ' : '';
    const title = `${dirty}${tab.fileName} \u2014 Text RS`;
    ipc.setWindowTitle(title).catch(() => {});
  }

  let lastTitleDirty = $state(false);

  $effect(() => {
    const tab = tabsStore.activeTab;
    if (!tab) {
      ipc.setWindowTitle('Text RS').catch(() => {});
      lastTitleDirty = false;
      return;
    }
    const dirty = tab.content !== tab.savedContent;
    if (dirty !== lastTitleDirty) {
      lastTitleDirty = dirty;
      updateWindowTitle();
    }
  });

  async function handleTabCloseRequest(e: CustomEvent<{ tabId: string }>) {
    const tabId = e.detail.tabId;
    const tab = tabsStore.tabs.find(t => t.id === tabId);
    if (!tab) return;

    if (tab.content === tab.savedContent) {
      tabsStore.forceCloseTab(tabId);
      return;
    }

    const result = await showConfirmDialog(
      'Save changes?',
      `"${tab.fileName}" has unsaved changes.`,
    );

    if (result === 'save') {
      if (tab.path) {
        try {
          await ipc.saveFile({
            path: tab.path,
            content: tab.content,
            lineEnding: tab.lineEnding,
            encoding: tab.encoding,
          });
          tabsStore.markSaved(tab.id, tab.path);
          tabsStore.forceCloseTab(tabId);
        } catch (e: unknown) {
          showToast(`Failed to save: ${errorMessage(e)}`);
        }
      } else {
        await handleSaveAs();
        // Close regardless of save-as result: user explicitly chose Save
        tabsStore.forceCloseTab(tabId);
      }
    } else if (result === 'discard') {
      tabsStore.forceCloseTab(tabId);
    }
  }

  /**
   * Set to true when we have already decided to close (after the user
   * confirmed via the dirty-tabs dialog). Prevents the close-interceptor
   * from intercepting the programmatic close() we then issue.
   */
  let isClosingProgrammatically = $state(false);

  /**
   * Close-window flow. Called from the Tauri close interceptor when there
   * ARE dirty tabs, or directly from Cmd+Q / the toolbar close button.
   * When there are no dirty tabs we let the OS close the window directly
   * (no preventDefault in the interceptor) to avoid an infinite loop.
   */
  async function handleCloseRequest(): Promise<void> {
    const dirtyTabs = tabsStore.getDirtyTabs();
    if (dirtyTabs.length === 0) {
      isClosingProgrammatically = true;
      getAppWindow().close();
      return;
    }

    const result = await showConfirmDialog(
      'Unsaved Changes',
      `You have ${dirtyTabs.length} unsaved file(s). Save before closing?`,
      { saveLabel: 'Save All' },
    );

    if (result === 'cancel') return;

    if (result === 'save') {
      const failed: Tab[] = [];
      for (const tab of dirtyTabs) {
        if (!tab.path) {
          failed.push(tab);
          continue;
        }
        try {
          await ipc.saveFile({
            path: tab.path,
            content: tab.content,
            lineEnding: tab.lineEnding,
            encoding: tab.encoding,
          });
          tabsStore.markSaved(tab.id, tab.path);
        } catch (e: unknown) {
          console.error('Failed to save:', e);
          failed.push(tab);
        }
      }
      if (failed.length > 0) {
        showToast(`Could not save ${failed.length} file(s). Close aborted.`);
        return;
      }
    }
    isClosingProgrammatically = true;
    getAppWindow().close();
  }

  function handleGoToLine() {
    const line = parseInt(goToLineValue, 10);
    if (isNaN(line) || line < 1) return;

    dispatchEditorAction({ action: 'go-to-line', line });
    showGoToLine = false;
    goToLineValue = '';
  }

  $effect(() => {
    if (showGoToLine) {
      (async () => {
        await tick();
        document.querySelector<HTMLInputElement>('.goto-line-input')?.focus();
      })();
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
          void handleTabCloseRequest(new CustomEvent('tab-close', { detail: { tabId: tab.id } }));
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
      dispatchEditorAction({ action: 'search-next', query: '', caseSensitive: false, useRegex: false });
    } else if (e.shiftKey && e.key === 'F3') {
      e.preventDefault();
      dispatchEditorAction({ action: 'search-prev', query: '', caseSensitive: false, useRegex: false });
    } else if (mod && !e.shiftKey && e.key === 'Tab') {
      e.preventDefault();
      const tabIds = tabsStore.tabs.map(t => t.id);
      if (tabIds.length === 0) return;
      const currentIdx = tabIds.indexOf(tabsStore.activeTabId ?? '');
      const nextIdx = currentIdx >= tabIds.length - 1 ? 0 : currentIdx + 1;
      tabsStore.setActive(tabIds[nextIdx]);
    } else if (mod && e.shiftKey && e.key === 'Tab') {
      e.preventDefault();
      const tabIds = tabsStore.tabs.map(t => t.id);
      if (tabIds.length === 0) return;
      const currentIdx = tabIds.indexOf(tabsStore.activeTabId ?? '');
      const prevIdx = currentIdx <= 0 ? tabIds.length - 1 : currentIdx - 1;
      tabsStore.setActive(tabIds[prevIdx]);
    } else if (mod && e.key >= '1' && e.key <= '9') {
      e.preventDefault();
      const idx = parseInt(e.key) - 1;
      if (idx < tabsStore.tabs.length) {
        tabsStore.setActive(tabsStore.tabs[idx].id);
      }
    } else if (mod && e.key === 'q') {
      // Cmd/Ctrl+Q — quit application. On macOS the OS usually reserves
      // this, but a custom handler here ensures the close-interceptor
      // (which checks for dirty tabs) runs.
      e.preventDefault();
      void handleCloseRequest();
    }
  }

  let recoveryInterval: ReturnType<typeof setInterval> | null = null;
  let lastRecoveryHash = '';

  async function saveRecovery() {
    const tabs = tabsStore.tabs.map(t => ({
      file_name: t.fileName,
      content: t.content,
      path: t.path,
      saved_at: new Date().toISOString(),
    }));
    // Skip the IPC roundtrip if nothing has changed since the last save
    const hash = JSON.stringify(tabs);
    if (hash === lastRecoveryHash) return;
    lastRecoveryHash = hash;
    try {
      await ipc.saveRecovery(tabs);
    } catch {
      // best-effort
    }
  }

  async function checkRecovery() {
    try {
      const entries = await ipc.checkRecovery();
      if (!entries || entries.length === 0) return;

      const names = entries.map(e => e.file_name).join(', ');
      const result = await showConfirmDialog(
        'Session Recovery',
        `Found unsaved files from previous session: ${names}. Restore them?`,
        { showDiscard: true, showCancel: true, saveLabel: 'Restore' },
      );

      if (result === 'save') {
        for (const entry of entries) {
          try {
            tabsStore.openTab({
              path: entry.path ?? '',
              content: entry.content,
              file_name: entry.file_name,
              encoding: 'UTF-8',
              line_ending: 'LF',
            });
          } catch (e: unknown) {
            showToast(`Could not restore ${entry.file_name}: ${errorMessage(e)}`);
          }
        }
        await ipc.clearRecovery();
      } else if (result === 'discard') {
        await ipc.clearRecovery();
      }
      // 'cancel' → leave recovery data in place for next launch
    } catch {
      // best-effort
    }
  }

  onMount(() => {
    // Single source of truth: Tauri onCloseRequested interceptor.
    // We MUST NOT call preventDefault unconditionally:
    //   1. If we do and the window has no dirty tabs, our own
    //      handleCloseRequest() will call getAppWindow().close(),
    //      which fires another onCloseRequested → we preventDefault
    //      again → infinite loop, the window never actually closes.
    //   2. If isClosingProgrammatically is set, we have already
    //      decided to close (e.g. user clicked "Don't Save"); let
    //      the OS proceed.
    //   3. Otherwise (no dirty tabs, OS-initiated), just return and
    //      let the OS close the window.
    //   4. If there ARE dirty tabs, preventDefault and ask.
    const closeUnlistenPromise = getAppWindow().onCloseRequested((event) => {
      if (isClosingProgrammatically) {
        // We initiated the close; allow it.
        return;
      }
      if (tabsStore.getDirtyTabs().length === 0) {
        // No dirty tabs; let the OS close the window.
        return;
      }
      // Has dirty tabs; intercept and show the dialog.
      event.preventDefault();
      void handleCloseRequest();
    });

    let pendingPollCount = 0;
    const POLL_INTERVAL_MS = 400;
    const MAX_POLL_COUNT = 30; // ~12 seconds at 400ms

    /**
     * Fallback poll for pending files. The `Opened` Apple Event /
     * `RunEvent::Opened` handler in Rust pushes files into a pending
     * list AND emits `file-opened`. The latter is event-based and
     * may race with the frontend's listener registration. The poll
     * is a deterministic fallback that drains any leftover files.
     */
    const pendingPollHandle = setInterval(() => {
      pendingPollCount++;
      void (async () => {
        try {
          const pending = await ipc.getPending();
          if (pending.length > 0) {
            console.log('[init] poll drained', pending.length, 'pending files');
            for (const filePath of pending) {
              await handleOpenFromPath(filePath);
            }
          }
        } catch {
          // ignore
        }
        if (pendingPollCount >= MAX_POLL_COUNT) {
          clearInterval(pendingPollHandle);
        }
      })();
    }, POLL_INTERVAL_MS);

    const init = async () => {
      // Run independent inits in parallel
      await Promise.all([settingsStore.init(), recentStore.refresh()]);

      if (tabsStore.tabs.length === 0) {
        tabsStore.newTab();
      }

      // Drain any pending files captured before the webview was ready
      // (argv + Apple Event capture on macOS). The list is consumed
      // by this call; subsequent pending files (e.g. RunEvent::Opened
      // that fires after the webview is loaded) come through the
      // `file-opened` Tauri event listener.
      try {
        const pending = await ipc.getPending();
        if (pending.length > 0) {
          console.log('[init] getPending returned', pending.length, 'files');
          for (const filePath of pending) {
            await handleOpenFromPath(filePath);
          }
        }
      } catch {
        // no pending files
      }

      // Tell Rust we are ready to receive file events. Rust will
      // re-emit any files it captured between `setup` and now as
      // `file-opened` Tauri events, so the listener (registered
      // in onMount above) can pick them up.
      try {
        const delivered = await ipc.frontendReady();
        if (delivered.length > 0) {
          console.log('[init] frontendReady delivered', delivered.length, 'files');
        }
      } catch {
        // ignore
      }

      await checkRecovery();
      updateWindowTitle();
    };
    void init();

    // Start recovery auto-save
    void saveRecovery();
    recoveryInterval = setInterval(() => void saveRecovery(), RECOVERY_INTERVAL_MS);

    // Keydown listener — on `document` with `capture: true` so it fires before
    // any focused element can stopPropagation. Handles shortcuts that the
    // native menu does NOT have an accelerator for (e.g. Ctrl+Tab, F3,
    // Ctrl+1..9). For shortcuts the menu DOES accelerate, see the Tauri
    // `listen()` calls below.
    const keydownListener = (e: KeyboardEvent) => handleGlobalKeydown(e);
    document.addEventListener('keydown', keydownListener, { capture: true });

    const tabCloseHandler: EventListener = (e) => {
      void handleTabCloseRequest(e as CustomEvent<{ tabId: string }>);
    };
    window.addEventListener('tab-close-request', tabCloseHandler);

    // Tauri menu events (emitted from Rust via window.emit). These are
    // NOT DOM events — they must be received via listen() from
    // @tauri-apps/api/event. The previous code used window.addEventListener
    // which never fired for these.
    const listen = listenTauriEvent;

    const listenPromises: Array<Promise<UnlistenFn>> = [
      listen('menu-new-tab', () => tabsStore.newTab()),
      listen('menu-open', () => void handleOpenFile()), // Native menu "Open..." has id "menu-open"
      listen('menu-save', () => void handleSave()),
      listen('menu-save-as', () => void handleSaveAs()),
      listen('menu-close-tab', () => {
        const tab = tabsStore.activeTab;
        if (!tab) return;
        if (tab.content !== tab.savedContent) {
          void handleTabCloseRequest(
            new CustomEvent('tab-close', { detail: { tabId: tab.id } }),
          );
        } else {
          tabsStore.forceCloseTab(tab.id);
        }
      }),
      listen('menu-undo', () => dispatchEditorAction({ action: 'undo' })),
      listen('menu-redo', () => dispatchEditorAction({ action: 'redo' })),
      listen('menu-cut', () => dispatchEditorAction({ action: 'cut' })),
      listen('menu-copy', () => dispatchEditorAction({ action: 'copy' })),
      listen('menu-paste', () => dispatchEditorAction({ action: 'paste' })),
      listen('menu-select-all', () => dispatchEditorAction({ action: 'select-all' })),
      listen('menu-find', () => { showFindReplace = true; }),
      listen('menu-find-replace', () => { showFindReplace = true; }),
      listen('menu-go-to-line', () => { showGoToLine = true; }),
      listen('menu-zoom-in', () => settingsStore.increaseFontSize()),
      listen('menu-zoom-out', () => settingsStore.decreaseFontSize()),
      listen('menu-zoom-reset', () => settingsStore.resetFontSize()),
      listen('menu-word-wrap', () => settingsStore.toggleWordWrap()),
      listen('menu-status-bar', () => settingsStore.toggleStatusBar()),
      listen<string>('menu-open-recent', (e) => { void handleOpenRecent(e.payload); }),
      listen('menu-about', () => {
        void showConfirmDialog(
          'About Text RS',
          'Text RS v1.0.0\nA fast, lightweight text editor.\nBuilt with Tauri, Svelte 5, and CodeMirror 6.',
          { showDiscard: false, showCancel: false, saveLabel: 'OK' },
        );
      }),
    ];

    // TabBar's right-click context menu emits DOM events (not Tauri events).
    // These are UI-internal, not native menu, so a DOM listener is correct.
    const contextMenuNewTab = () => tabsStore.newTab();
    const contextMenuOpenFile = () => void handleOpenFile();
    const contextMenuSave = () => void handleSave();
    const contextMenuSaveAs = () => void handleSaveAs();
    const contextMenuCloseTab = () => {
      const tab = tabsStore.activeTab;
      if (!tab) return;
      if (tab.content !== tab.savedContent) {
        void handleTabCloseRequest(
          new CustomEvent('tab-close', { detail: { tabId: tab.id } }),
        );
      } else {
        tabsStore.forceCloseTab(tab.id);
      }
    };
    const contextMenuUndo = () => dispatchEditorAction({ action: 'undo' });
    const contextMenuRedo = () => dispatchEditorAction({ action: 'redo' });
    const contextMenuCut = () => dispatchEditorAction({ action: 'cut' });
    const contextMenuCopy = () => dispatchEditorAction({ action: 'copy' });
    const contextMenuPaste = () => dispatchEditorAction({ action: 'paste' });
    const contextMenuSelectAll = () => dispatchEditorAction({ action: 'select-all' });
    const contextMenuFind = () => { showFindReplace = true; };
    const contextMenuFindReplace = () => { showFindReplace = true; };
    const contextMenuGoToLine = () => { showGoToLine = true; };
    const contextMenuZoomIn = () => settingsStore.increaseFontSize();
    const contextMenuZoomOut = () => settingsStore.decreaseFontSize();
    const contextMenuZoomReset = () => settingsStore.resetFontSize();
    const contextMenuWordWrap = () => settingsStore.toggleWordWrap();
    const contextMenuStatusBar = () => settingsStore.toggleStatusBar();

    window.addEventListener('menu-new-tab', contextMenuNewTab);
    window.addEventListener('menu-open-file', contextMenuOpenFile);
    window.addEventListener('menu-save', contextMenuSave);
    window.addEventListener('menu-save-as', contextMenuSaveAs);
    window.addEventListener('menu-close-tab', contextMenuCloseTab);
    window.addEventListener('menu-undo', contextMenuUndo);
    window.addEventListener('menu-redo', contextMenuRedo);
    window.addEventListener('menu-cut', contextMenuCut);
    window.addEventListener('menu-copy', contextMenuCopy);
    window.addEventListener('menu-paste', contextMenuPaste);
    window.addEventListener('menu-select-all', contextMenuSelectAll);
    window.addEventListener('menu-find', contextMenuFind);
    window.addEventListener('menu-find-replace', contextMenuFindReplace);
    window.addEventListener('menu-go-to-line', contextMenuGoToLine);
    window.addEventListener('menu-zoom-in', contextMenuZoomIn);
    window.addEventListener('menu-zoom-out', contextMenuZoomOut);
    window.addEventListener('menu-zoom-reset', contextMenuZoomReset);
    window.addEventListener('menu-word-wrap', contextMenuWordWrap);
    window.addEventListener('menu-status-bar', contextMenuStatusBar);

    // Tauri file-opened events and drag-drop
    const unlistenFileOpened = getAppWindow().listen<string[]>('file-opened', async (event) => {
      for (const filePath of event.payload) {
        await handleOpenFromPath(filePath);
      }
    });
    const unlistenDragDrop = getAppWindow().onDragDropEvent(async (event) => {
      if (event.payload.type === 'drop') {
        for (const filePath of event.payload.paths) {
          await handleOpenFromPath(filePath);
        }
      }
    });

    return () => {
      if (toastTimer) clearTimeout(toastTimer);
      if (recoveryInterval) clearInterval(recoveryInterval);
      clearInterval(pendingPollHandle);
      document.removeEventListener('keydown', keydownListener, { capture: true });
      window.removeEventListener('tab-close-request', tabCloseHandler);
      // Context menu DOM listeners
      window.removeEventListener('menu-new-tab', contextMenuNewTab);
      window.removeEventListener('menu-open-file', contextMenuOpenFile);
      window.removeEventListener('menu-save', contextMenuSave);
      window.removeEventListener('menu-save-as', contextMenuSaveAs);
      window.removeEventListener('menu-close-tab', contextMenuCloseTab);
      window.removeEventListener('menu-undo', contextMenuUndo);
      window.removeEventListener('menu-redo', contextMenuRedo);
      window.removeEventListener('menu-cut', contextMenuCut);
      window.removeEventListener('menu-copy', contextMenuCopy);
      window.removeEventListener('menu-paste', contextMenuPaste);
      window.removeEventListener('menu-select-all', contextMenuSelectAll);
      window.removeEventListener('menu-find', contextMenuFind);
      window.removeEventListener('menu-find-replace', contextMenuFindReplace);
      window.removeEventListener('menu-go-to-line', contextMenuGoToLine);
      window.removeEventListener('menu-zoom-in', contextMenuZoomIn);
      window.removeEventListener('menu-zoom-out', contextMenuZoomOut);
      window.removeEventListener('menu-zoom-reset', contextMenuZoomReset);
      window.removeEventListener('menu-word-wrap', contextMenuWordWrap);
      window.removeEventListener('menu-status-bar', contextMenuStatusBar);
      void Promise.all(listenPromises).then((fns) => fns.forEach((fn) => fn()));
      unlistenFileOpened.then((fn) => fn());
      unlistenDragDrop.then((fn) => fn());
      closeUnlistenPromise.then((fn) => fn());
    };
  });
</script>

<div class="app">
  <TabBar />
  <div class="editor-area">
    {#if activeTab}
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
      {#key activeTab?.id ?? 'empty'}
        <Editor
          tabId={activeTab?.id ?? ''}
          content={activeTab?.content ?? ''}
          language={activeTab?.language ?? 'text'}
          onContentChange={handleContentChange}
          onCursorUpdate={handleCursorUpdate}
        />
      {/key}
    {:else}
      <div class="empty-state">
        <p>No open files</p>
        <p class="empty-hint">Ctrl+N to create a new tab, Ctrl+O to open a file</p>
      </div>
    {/if}
  </div>
  <StatusBar />

  {#if showRecentDialog}
    <div class="toast-backdrop" onclick={() => showRecentDialog = false} onkeydown={() => {}} role="presentation">
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
