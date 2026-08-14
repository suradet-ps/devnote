<script lang="ts">
  import { onMount, tick } from 'svelte';
  import TabBar from '$lib/components/TabBar.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import FindReplace from '$lib/components/FindReplace.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import SymbolPicker from '$lib/components/SymbolPicker.svelte';
  import EncodingPicker from '$lib/components/EncodingPicker.svelte';
  import type { SymbolInfo } from '$lib/editor/symbols';
  import { tabsStore, type Tab } from '$lib/stores/tabs.svelte';
  import { recentStore } from '$lib/stores/recent.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { ipc } from '$lib/tauri/ipc';
  import { dispatchEditorAction } from '$lib/editor/actions';
  import { errorMessage } from '$lib/utils/error';
  import { recoveryHash } from '$lib/utils/recovery';
  import { t } from '$lib/i18n/i18n.svelte';
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
  let showIndentGuides = $state(false);
  let showVisibleWhitespace = $state(false);
  let showSymbolPicker = $state(false);
  let symbolList = $state<SymbolInfo[]>([]);
  let showPrintOverlay = $state(false);
  let printContent = $state('');
  let showEncodingPicker = $state(false);
  let encodingPickerFileName = $state('');
  let encodingPickerDetected = $state('');
  let encodingResolve: ((enc: string | null) => void) | null = null;

  function showEncodingChoice(fileName: string, detected: string): Promise<string | null> {
    return new Promise((resolve) => {
      encodingPickerFileName = fileName;
      encodingPickerDetected = detected;
      encodingResolve = resolve;
      showEncodingPicker = true;
    });
  }

  function resolveEncodingChoice(enc: string | null) {
    showEncodingPicker = false;
    const r = encodingResolve;
    encodingResolve = null;
    r?.(enc);
  }

  /** Open a payload, prompting for an encoding override when confidence is low. */
  async function openPayload(payload: FilePayload): Promise<void> {
    if (payload.encoding_confident === false) {
      const enc = await showEncodingChoice(payload.file_name, payload.encoding);
      if (!enc) return; // user cancelled the open
      if (enc !== payload.encoding) {
        try {
          payload = await ipc.readFileWithEncoding(payload.path, enc);
        } catch (e: unknown) {
          showToast(t('toast.openFailedGeneric', { error: errorMessage(e) }));
          return;
        }
      }
    }
    tabsStore.openTab(payload);
    watchPath(payload.path);
    await recentStore.add(payload.path);
  }

  async function handlePrint() {
    const tab = tabsStore.activeTab;
    if (!tab) return;
    printContent = tab.content;
    showPrintOverlay = true;
    await tick();
    try {
      await ipc.printCurrent();
    } catch (e) {
      console.error('print failed', e);
      showToast(t('print.failed'));
    } finally {
      showPrintOverlay = false;
    }
  }
  let recentDialogEl = $state<HTMLDivElement | null>(null);
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
    discardLabel: string;
    resolve: (r: ConfirmResult) => void;
  }
  let confirmOpen = $state(false);
  let confirmTitle = $state('');
  let confirmMessage = $state('');
  let confirmShowSave = $state(true);
  let confirmShowDiscard = $state(true);
  let confirmShowCancel = $state(true);
  let confirmSaveLabel = $state(t('dialog.save'));
  let confirmDiscardLabel = $state(t('dialog.dontSave'));
  let confirmQueue: ConfirmRequest[] = $state([]);

  function showConfirmDialog(
    title: string,
    message: string,
    opts?: { saveLabel?: string; discardLabel?: string; showSave?: boolean; showDiscard?: boolean; showCancel?: boolean }
  ): Promise<ConfirmResult> {
    return new Promise<ConfirmResult>((resolve) => {
      const req: ConfirmRequest = {
        title,
        message,
        showSave: opts?.showSave ?? true,
        showDiscard: opts?.showDiscard ?? true,
        showCancel: opts?.showCancel ?? true,
        saveLabel: opts?.saveLabel ?? t('dialog.save'),
        discardLabel: opts?.discardLabel ?? t('dialog.dontSave'),
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
    confirmDiscardLabel = req.discardLabel;
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

  function watchPath(path: string | null) {
    if (!path) return;
    ipc.watchFile(path).catch(() => {});
  }

  function unwatchPath(path: string | null) {
    if (!path) return;
    ipc.unwatchFile(path).catch(() => {});
  }

  /** External-change event from the watcher: prompt Reload / Ignore. */
  async function handleFileChangedExternal(path: string) {
    const tab = tabsStore.tabs.find((t) => t.path === path);
    if (!tab) {
      // Tab is gone — stop watching
      await ipc.unwatchFile(path).catch(() => {});
      return;
    }
    const dirty = tab.content !== tab.savedContent;
    const result = await showConfirmDialog(
      t('dialog.fileChangedTitle'),
      t('dialog.fileChangedBody', {
        name: tab.fileName,
        dirty: dirty ? t('dialog.fileChangedDirty') : '',
      }),
      { showDiscard: true, showCancel: true, saveLabel: t('dialog.fileChangedReload'), discardLabel: t('dialog.fileChangedIgnore') },
    );
    if (result !== 'save') return; // Ignore / Cancel — next change re-prompts
    try {
      const payload = await ipc.readFile(path);
      tabsStore.reloadTab(tab.id, payload);
      updateWindowTitle();
    } catch (e: unknown) {
      showToast(t('toast.openFailedGeneric', { error: errorMessage(e) }));
    }
  }

  async function handleOpenFile() {
    try {
      const payload = await ipc.openFile();
      if (payload) {
        await openPayload(payload);
      }
    } catch (e: unknown) {
      showToast(t('toast.openFailed', { error: errorMessage(e) }));
    }
  }

  async function handleOpenRecent(path: string) {
    try {
      const payload = await ipc.readFile(path);
      await openPayload(payload);
    } catch (e: unknown) {
      const err = errorMessage(e);
      if (err.toLowerCase().includes('not found') || err.toLowerCase().includes('no such file')) {
        await recentStore.remove(path);
        showToast(t('toast.fileNotFound', { path }));
      } else {
        showToast(t('toast.openFailedGeneric', { error: err }));
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
          t('dialog.largeFileTitle'),
          t('dialog.largeFileBody', { size: (size / (1024 * 1024)).toFixed(1) }),
        );
        if (result !== 'save') return;
      }
      const payload = await ipc.readFile(path);
      await openPayload(payload);
    } catch (e: unknown) {
      showToast(t('toast.openFailedGeneric', { error: errorMessage(e) }));
    }
  }

  async function handleSave() {
    const tab = tabsStore.activeTab;
    if (!tab) return;

    if (tab.readOnly) {
      showToast(t('toast.readOnly'));
      return;
    }

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
          t('dialog.cannotSaveTitle'),
          t('dialog.cannotSaveBody', { name: tab.fileName }),
          { showDiscard: false, saveLabel: t('dialog.saveCopy') },
        );
        if (result === 'save') {
          await handleSaveAs();
        }
      } else {
        showToast(t('toast.saveFailed', { error: err }));
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
        unwatchPath(tab.path);
        tabsStore.markSaved(tab.id, newPath);
        watchPath(newPath);
        await recentStore.add(newPath);
        updateWindowTitle();
      }
    } catch (e: unknown) {
      showToast(t('toast.saveAsFailed', { error: errorMessage(e) }));
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
      ipc.setWindowTitle('DevNote').catch(() => {});
      return;
    }
    const dirty = tab.content !== tab.savedContent ? '\u2022 ' : '';
    const title = `${dirty}${tab.fileName} \u2014 DevNote`;
    ipc.setWindowTitle(title).catch(() => {});
  }

  let lastTitleDirty = $state(false);

  $effect(() => {
    const tab = tabsStore.activeTab;
    if (!tab) {
      ipc.setWindowTitle('DevNote').catch(() => {});
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
    const closingPath = tab.path;

    if (tab.content === tab.savedContent) {
      tabsStore.forceCloseTab(tabId);
      unwatchPath(closingPath);
      return;
    }

    const result = await showConfirmDialog(
      t('dialog.saveChangesTitle'),
      t('dialog.saveChangesBody', { name: tab.fileName }),
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
          unwatchPath(closingPath);
        } catch (e: unknown) {
          showToast(t('toast.saveFailed', { error: errorMessage(e) }));
        }
      } else {
        await handleSaveAs();
        // Close regardless of save-as result: user explicitly chose Save
        tabsStore.forceCloseTab(tabId);
      }
    } else if (result === 'discard') {
      tabsStore.forceCloseTab(tabId);
      unwatchPath(closingPath);
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
      t('dialog.unsavedTitle'),
      t('dialog.unsavedBody', { count: dirtyTabs.length }),
      { saveLabel: t('dialog.saveAll') },
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
        showToast(t('toast.saveAborted', { count: failed.length }));
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

  // Focus the recent-files dialog when it opens
  $effect(() => {
    if (showRecentDialog) {
      void (async () => {
        await tick();
        recentDialogEl
          ?.querySelector<HTMLButtonElement>('.recent-item, .recent-empty')
          ?.focus();
        recentDialogEl?.focus();
      })();
    }
  });

  function handleGlobalKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;

    // Text-editing shortcuts must not hijack native editing while focus is
    // in an input (find/replace, go-to-line, tab rename, symbol picker).
    // Ctrl+V in the find field should paste into the field, not the editor.
    const target = e.target as HTMLElement | null;
    const inInput = !!target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
    if (inInput && mod && (e.key === 'v' || e.key === 'c' || e.key === 'x' || e.key === 'a' || e.key === 'z')) {
      return;
    }

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
    } else if (mod && !e.altKey && e.key === '=') {
      e.preventDefault();
      settingsStore.increaseFontSize();
    } else if (mod && !e.altKey && e.key === '-') {
      e.preventDefault();
      settingsStore.decreaseFontSize();
    } else if (mod && !e.altKey && e.key === '0') {
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
    } else if (mod && !e.shiftKey && e.key === 'd') {
      e.preventDefault();
      dispatchEditorAction({ action: 'add-next-occurrence' });
    } else if (mod && e.shiftKey && e.key === 'l') {
      e.preventDefault();
      dispatchEditorAction({ action: 'select-all-occurrences' });
    } else if (mod && e.shiftKey && e.key === 'p') {
      e.preventDefault();
      dispatchEditorAction({ action: 'go-to-symbol' });
    } else if (mod && !e.shiftKey && e.key === 'p') {
      e.preventDefault();
      void handlePrint();
    } else if (mod && e.altKey && e.key === '-') {
      e.preventDefault();
      dispatchEditorAction({ action: 'jump-edit-back' });
    } else if (mod && e.altKey && e.key === '=') {
      e.preventDefault();
      dispatchEditorAction({ action: 'jump-edit-forward' });
    } else if (mod && e.key === 'z' && !e.shiftKey) {
      e.preventDefault();
      dispatchEditorAction({ action: 'undo' });
    } else if (mod && e.key === 'z' && e.shiftKey) {
      e.preventDefault();
      dispatchEditorAction({ action: 'redo' });
    } else if (mod && e.key === 'c') {
      e.preventDefault();
      dispatchEditorAction({ action: 'copy' });
    } else if (mod && e.key === 'x') {
      e.preventDefault();
      dispatchEditorAction({ action: 'cut' });
    } else if (mod && e.key === 'v') {
      e.preventDefault();
      dispatchEditorAction({ action: 'paste' });
    } else if (mod && e.key === 'a') {
      e.preventDefault();
      dispatchEditorAction({ action: 'select-all' });
    }
  }

  let recoveryInterval: ReturnType<typeof setInterval> | null = null;
  let lastRecoveryHash = '';

  async function saveRecovery() {
    // Read-only previews can't have unsaved work and are too big to write
    // out every 15 s — exclude them from recovery.
    const tabs = tabsStore.tabs.filter(t => !t.readOnly).map(t => ({
      file_name: t.fileName,
      content: t.content,
      path: t.path,
      saved_at: new Date().toISOString(),
    }));
    // Skip the IPC roundtrip if nothing has changed since the last save
    const hash = recoveryHash(tabs);
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
        t('recovery.title'),
        t('recovery.body', { names }),
        { showDiscard: true, showCancel: true, saveLabel: t('recovery.restore') },
      );

      if (result === 'save') {
        for (const entry of entries) {
          try {
            const restored = tabsStore.openTab({
              path: entry.path ?? '',
              content: entry.content,
              file_name: entry.file_name,
              encoding: 'UTF-8',
              line_ending: 'LF',
            });
            watchPath(restored.path);
          } catch (e: unknown) {
            showToast(t('toast.restoreFailed', { name: entry.file_name, error: errorMessage(e) }));
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

    const symbolsReadyHandler: EventListener = (e) => {
      symbolList = (e as CustomEvent<SymbolInfo[]>).detail ?? [];
      showSymbolPicker = true;
    };
    window.addEventListener('symbols-ready', symbolsReadyHandler);

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
      listen('menu-add-next-occurrence', () => dispatchEditorAction({ action: 'add-next-occurrence' })),
      listen('menu-select-all-occurrences', () => dispatchEditorAction({ action: 'select-all-occurrences' })),
      listen('menu-find', () => { showFindReplace = true; }),
      listen('menu-find-replace', () => { showFindReplace = true; }),
      listen('menu-go-to-line', () => { showGoToLine = true; }),
      listen('menu-zoom-in', () => settingsStore.increaseFontSize()),
      listen('menu-zoom-out', () => settingsStore.decreaseFontSize()),
      listen('menu-zoom-reset', () => settingsStore.resetFontSize()),
      listen('menu-jump-edit-back', () => dispatchEditorAction({ action: 'jump-edit-back' })),
      listen('menu-jump-edit-forward', () => dispatchEditorAction({ action: 'jump-edit-forward' })),
      listen('menu-go-to-symbol', () => dispatchEditorAction({ action: 'go-to-symbol' })),
      listen('menu-print', () => void handlePrint()),
      listen('menu-word-wrap', () => settingsStore.toggleWordWrap()),
      listen('menu-status-bar', () => settingsStore.toggleStatusBar()),
      listen('menu-indent-guides', () => { showIndentGuides = !showIndentGuides; }),
      listen('menu-visible-whitespace', () => { showVisibleWhitespace = !showVisibleWhitespace; }),
      listen<string>('menu-open-recent', (e) => { void handleOpenRecent(e.payload); }),
      listen<string>('file-changed-external', (e) => { void handleFileChangedExternal(e.payload); }),
      listen('menu-about', () => {
        void showConfirmDialog(
          t('about.title'),
          t('about.body'),
          { showDiscard: false, showCancel: false, saveLabel: t('dialog.ok') },
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
      window.removeEventListener('symbols-ready', symbolsReadyHandler);
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
      <SymbolPicker
        open={showSymbolPicker}
        symbols={symbolList}
        onSelect={(s) => {
          showSymbolPicker = false;
          dispatchEditorAction({ action: 'jump-to-symbol', line: s.line });
        }}
        onClose={() => showSymbolPicker = false}
      />
      {#if showGoToLine}
        <div class="goto-line-panel" role="dialog" aria-label={t('goto.label')}>
          <input
            class="goto-line-input"
            type="number"
            min="1"
            placeholder={t('goto.placeholder')}
            bind:value={goToLineValue}
            onkeydown={(e) => {
              if (e.key === 'Enter') handleGoToLine();
              if (e.key === 'Escape') { showGoToLine = false; goToLineValue = ''; }
            }}
          />
          <button class="goto-line-btn" onclick={handleGoToLine}>{t('goto.go')}</button>
        </div>
      {/if}
      {#key activeTab?.id ?? 'empty'}
        <Editor
          tabId={activeTab?.id ?? ''}
          content={activeTab?.content ?? ''}
          language={activeTab?.language ?? 'text'}
          indentGuides={showIndentGuides}
          visibleWhitespace={showVisibleWhitespace}
          readOnly={activeTab?.readOnly ?? false}
          onContentChange={handleContentChange}
          onCursorUpdate={handleCursorUpdate}
        />
      {/key}
    {:else}
      <div class="empty-state">
        <p>{t('empty.noOpenFiles')}</p>
        <p class="empty-hint">{t('empty.hint')}</p>
      </div>
    {/if}
  </div>
  <StatusBar />

  {#if showRecentDialog}
    <div class="toast-backdrop" onclick={() => showRecentDialog = false} role="presentation">
      <div
        class="recent-dialog"
        bind:this={recentDialogEl}
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => { if (e.key === 'Escape') showRecentDialog = false; }}
        role="dialog"
        aria-modal="true"
        aria-label={t('recent.title')}
        tabindex="-1"
      >
        <h3>{t('recent.title')}</h3>
        {#if recentStore.recentFiles.length === 0}
          <p class="recent-empty">{t('recent.empty')}</p>
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

{#if showPrintOverlay}
  <div class="print-overlay">
    <div class="print-header">
      <span class="print-title">{t('print.title')} — {activeTab?.fileName ?? ''}</span>
      <button class="print-cancel" onclick={() => showPrintOverlay = false}>{t('dialog.cancel')}</button>
    </div>
    <pre class="print-body">{printContent}</pre>
  </div>
{/if}

<ConfirmDialog
  open={confirmOpen}
  title={confirmTitle}
  message={confirmMessage}
  showSave={confirmShowSave}
  showDiscard={confirmShowDiscard}
  showCancel={confirmShowCancel}
  saveLabel={confirmSaveLabel}
  discardLabel={confirmDiscardLabel}
  onSave={handleConfirmSave}
  onDiscard={handleConfirmDiscard}
  onCancel={handleConfirmCancel}
/>

{#if showEncodingPicker}
  <EncodingPicker
    open={showEncodingPicker}
    fileName={encodingPickerFileName}
    detected={encodingPickerDetected}
    onSelect={(enc) => resolveEncodingChoice(enc)}
    onClose={() => resolveEncodingChoice(null)}
  />
{/if}

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

  .print-overlay {
    position: fixed;
    inset: 0;
    background: #fff;
    color: #111;
    z-index: 900;
    display: flex;
    flex-direction: column;
  }

  .print-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-sm) var(--sp-md);
    border-bottom: 1px solid var(--hairline);
    font-size: 13px;
    color: var(--muted);
  }

  .print-title {
    font-weight: 500;
    color: var(--body);
  }

  .print-cancel {
    padding: var(--sp-xxs) var(--sp-sm);
    border-radius: var(--r-sm);
    color: var(--muted);
  }

  .print-cancel:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }

  .print-body {
    flex: 1;
    overflow: auto;
    padding: var(--sp-lg);
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  @media print {
    .app {
      display: none;
    }

    .print-overlay {
      position: static;
    }

    .print-header {
      display: none;
    }

    .print-body {
      overflow: visible;
      padding: 0;
    }
  }
</style>
