<script lang="ts">
  import Tab from './Tab.svelte';
  import { tabsStore } from '$lib/stores/tabs.svelte';

  let showContextMenu = $state(false);
  let contextMenuPos = $state({ x: 0, y: 0 });
  let contextMenuTabId = $state('');

  function handleNewTab() {
    tabsStore.newTab();
  }

  function handleTabClick(id: string) {
    tabsStore.setActive(id);
  }

  function handleTabClose(id: string) {
    const tab = tabsStore.tabs.find(t => t.id === id);
    if (tab && tab.content !== tab.savedContent) {
      window.dispatchEvent(new CustomEvent('tab-close-request', { detail: { tabId: id } }));
    } else {
      tabsStore.forceCloseTab(id);
    }
  }

  function handleContextMenu(e: MouseEvent, tabId: string) {
    contextMenuPos = { x: e.clientX, y: e.clientY };
    contextMenuTabId = tabId;
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
  }

  function emit(name: string, detail?: unknown) {
    window.dispatchEvent(new CustomEvent(name, { detail }));
  }

  function closeOtherTabs() {
    tabsStore.tabs.forEach(t => {
      if (t.id !== contextMenuTabId && t.content === t.savedContent) {
        tabsStore.forceCloseTab(t.id);
      }
    });
    closeContextMenu();
  }

  function closeAllTabs() {
    tabsStore.tabs.forEach(t => {
      if (t.content === t.savedContent) {
        tabsStore.forceCloseTab(t.id);
      }
    });
    closeContextMenu();
  }

  function copyPath() {
    const tab = tabsStore.tabs.find(t => t.id === contextMenuTabId);
    if (tab?.path) {
      navigator.clipboard.writeText(tab.path);
    }
    closeContextMenu();
  }

  function renameTab() {
    const tab = tabsStore.tabs.find(t => t.id === contextMenuTabId);
    if (tab) {
      const name = prompt('Rename tab:', tab.fileName);
      if (name && name.trim() && name.trim() !== tab.fileName) {
        tabsStore.renameTab(tab.id, name.trim());
      }
    }
    closeContextMenu();
  }
</script>

<div class="tabbar">
  <div class="tabbar-scroll">
    {#each tabsStore.tabs as tab (tab.id)}
      <Tab
        fileName={tab.fileName}
        isDirty={tab.content !== tab.savedContent}
        isActive={tab.id === tabsStore.activeTabId}
        onclick={() => handleTabClick(tab.id)}
        onclose={() => handleTabClose(tab.id)}
        oncontextmenu={(e) => handleContextMenu(e, tab.id)}
        onrename={(name) => tabsStore.renameTab(tab.id, name)}
      />
    {/each}
  </div>
  <button class="tabbar-new" onclick={handleNewTab} title="New tab (Ctrl+N)">
    <svg width="14" height="14" viewBox="0 0 14 14">
      <path fill="currentColor" d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    </svg>
  </button>
</div>

{#if showContextMenu}
  <div
    class="context-menu-overlay"
    onclick={closeContextMenu}
    onkeydown={(e) => e.key === 'Escape' && closeContextMenu()}
    role="button"
    tabindex="-1"
  >
    <div
      class="context-menu"
      style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px;"
      onclick={(e) => e.stopPropagation()}
      onkeydown={() => {}}
      role="menu"
      tabindex="-1"
    >
      <button class="context-menu-item" onclick={() => { emit('menu-new-tab'); closeContextMenu(); }}>
        <span>New Tab</span>
        <span class="shortcut">Ctrl+N</span>
      </button>
      <button class="context-menu-item" onclick={() => { emit('menu-open-file'); closeContextMenu(); }}>
        <span>Open File...</span>
        <span class="shortcut">Ctrl+O</span>
      </button>
      <div class="context-menu-sep"></div>
      <button class="context-menu-item" onclick={() => { emit('menu-save'); closeContextMenu(); }}>
        <span>Save</span>
        <span class="shortcut">Ctrl+S</span>
      </button>
      <button class="context-menu-item" onclick={() => { emit('menu-save-as'); closeContextMenu(); }}>
        <span>Save As...</span>
        <span class="shortcut">Ctrl+Shift+S</span>
      </button>
      <div class="context-menu-sep"></div>
      <button class="context-menu-item" onclick={() => { handleTabClose(contextMenuTabId); closeContextMenu(); }}>
        <span>Close</span>
        <span class="shortcut">Ctrl+W</span>
      </button>
      <button class="context-menu-item" onclick={closeOtherTabs}>
        Close Others
      </button>
      <button class="context-menu-item" onclick={closeAllTabs}>
        Close All
      </button>
      <button class="context-menu-item" onclick={renameTab}>
        Rename
      </button>
      {#if tabsStore.tabs.find(t => t.id === contextMenuTabId)?.path}
        <div class="context-menu-sep"></div>
        <button class="context-menu-item" onclick={copyPath}>
          Copy Path
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .tabbar {
    height: 32px;
    background: var(--surface-soft);
    display: flex;
    align-items: stretch;
    flex-shrink: 0;
    overflow: hidden;
    border-bottom: 1px solid var(--hairline);
  }

  .tabbar-scroll {
    display: flex;
    overflow-x: auto;
    scrollbar-width: none;
    flex: 1;
  }

  .tabbar-scroll::-webkit-scrollbar {
    display: none;
  }

  .tabbar-new {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    flex-shrink: 0;
    transition: color 0.15s, background 0.15s;
  }

  .tabbar-new:hover {
    color: var(--ink);
    background: var(--surface-card);
  }

  .context-menu-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
  }

  .context-menu {
    position: fixed;
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-md);
    padding: var(--sp-xxs) 0;
    min-width: 200px;
    box-shadow: 0 4px 16px rgba(20, 20, 19, 0.15);
    z-index: 201;
  }

  .context-menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 6px var(--sp-md);
    font-size: 13px;
    color: var(--ink);
    text-align: left;
    transition: background 0.1s;
  }

  .context-menu-item:hover {
    background: var(--surface-soft);
  }

  .shortcut {
    font-size: 11px;
    color: var(--muted-soft);
    margin-left: var(--sp-lg);
  }

  .context-menu-sep {
    height: 1px;
    background: var(--hairline);
    margin: var(--sp-xxs) 0;
  }
</style>
