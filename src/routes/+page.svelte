<script lang="ts">
  import { onMount } from 'svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import TabBar from '$lib/components/TabBar.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import FindReplace from '$lib/components/FindReplace.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { tabsStore } from '$lib/stores/tabs';
  import { recentStore } from '$lib/stores/recent';
  import { settingsStore } from '$lib/stores/settings';
  import { invoke } from '@tauri-apps/api/core';
  import type { FilePayload } from '$lib/stores/tabs';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const appWindow = getCurrentWindow();

  let showFindReplace = $state(false);
  let findReplaceMode = $state<'find' | 'replace'>('find');

  let confirmOpen = $state(false);
  let confirmTitle = $state('');
  let confirmMessage = $state('');
  let confirmAction = $state<'save' | 'discard' | 'cancel'>('cancel');
  let confirmResolve: ((value: string) => void) | null = null;

  function showConfirmDialog(title: string, message: string): Promise<string> {
    return new Promise((resolve) => {
      confirmTitle = title;
      confirmMessage = message;
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

  async function handleOpenFile() {
    try {
      const payload = await invoke<FilePayload | null>('open_file');
      if (payload) {
        tabsStore.openTab(payload);
        await recentStore.add(payload.path);
      }
    } catch (e) {
      console.error('Failed to open file:', e);
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
    } catch (e) {
      console.error('Failed to save:', e);
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
      }
    } catch (e) {
      console.error('Failed to save as:', e);
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

  async function handleTabCloseRequest(e: CustomEvent) {
    const tabId = e.detail.tabId;
    const tab = tabsStore.tabs.find(t => t.id === tabId);
    if (!tab) return;

    const result = await showConfirmDialog(
      'Save changes?',
      `"${tab.fileName}" has unsaved changes.`
    );

    if (result === 'save') {
      if (tab.path) {
        try {
          await invoke('save_file', { path: tab.path, content: tab.content });
          tabsStore.markSaved(tab.id, tab.path);
          tabsStore.forceCloseTab(tabId);
        } catch (e) {
          console.error('Failed to save:', e);
        }
      } else {
        await handleSaveAs();
        tabsStore.forceCloseTab(tabId);
      }
    } else if (result === 'discard') {
      tabsStore.forceCloseTab(tabId);
    }
  }

  async function handleWindowCloseRequest() {
    const dirtyTabs = tabsStore.getDirtyTabs();
    if (dirtyTabs.length === 0) {
      appWindow.close();
      return;
    }

    const result = await showConfirmDialog(
      'Save changes?',
      `You have ${dirtyTabs.length} unsaved file(s). Save before closing?`
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
      appWindow.close();
    } else if (result === 'discard') {
      appWindow.close();
    }
  }

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
      findReplaceMode = 'find';
      showFindReplace = !showFindReplace;
    } else if (mod && e.key === 'h') {
      e.preventDefault();
      findReplaceMode = 'replace';
      showFindReplace = true;
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
    } else if (e.key === 'Tab' && !mod) {
      const tabIds = tabsStore.tabs.map(t => t.id);
      const currentIdx = tabIds.indexOf(tabsStore.activeTabId ?? '');
      if (e.shiftKey) {
        const prevIdx = currentIdx <= 0 ? tabIds.length - 1 : currentIdx - 1;
        tabsStore.setActive(tabIds[prevIdx]);
      } else {
        const nextIdx = currentIdx >= tabIds.length - 1 ? 0 : currentIdx + 1;
        tabsStore.setActive(tabIds[nextIdx]);
      }
    } else if (mod && e.key >= '1' && e.key <= '9') {
      const idx = parseInt(e.key) - 1;
      if (idx < tabsStore.tabs.length) {
        tabsStore.setActive(tabsStore.tabs[idx].id);
      }
    }
  }

  function handleFindReplaceFind(query: string) {
    // TODO: integrate with CodeMirror search
  }

  function handleFindReplaceReplace(query: string, replacement: string) {
    // TODO: integrate with CodeMirror search
  }

  onMount(() => {
    recentStore.refresh();
    if (tabsStore.tabs.length === 0) {
      tabsStore.newTab();
    }

    window.addEventListener('keydown', handleGlobalKeydown);
    window.addEventListener('tab-close-request', handleTabCloseRequest as unknown as EventListener);
    window.addEventListener('window-close-request', handleWindowCloseRequest);

    const unlisten = appWindow.onCloseRequested(async (event) => {
      event.preventDefault();
      await handleWindowCloseRequest();
    });

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
      window.removeEventListener('tab-close-request', handleTabCloseRequest as unknown as EventListener);
      window.removeEventListener('window-close-request', handleWindowCloseRequest);
    };
  });
</script>

<div class="app">
  <TitleBar />
  <TabBar />
  <div class="editor-area">
    {#if tabsStore.activeTab}
      <FindReplace
        show={showFindReplace}
        onClose={() => showFindReplace = false}
        onFind={handleFindReplaceFind}
        onReplace={handleFindReplaceReplace}
      />
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
</div>

<ConfirmDialog
  open={confirmOpen}
  title={confirmTitle}
  message={confirmMessage}
  onSave={handleConfirmSave}
  onDiscard={handleConfirmDiscard}
  onCancel={handleConfirmCancel}
/>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--surface-dark);
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
</style>
