<script lang="ts">
  import Tab from './Tab.svelte';
  import { tabsStore } from '$lib/stores/tabs.svelte';
  import { t } from '$lib/i18n/i18n.svelte';

  let showContextMenu = $state(false);
  let contextMenuPos = $state({ x: 0, y: 0 });
  let contextMenuTabId = $state('');
  let contextMenuEl = $state<HTMLDivElement | null>(null);
  let contextMenuReturnFocus: HTMLElement | null = null;
  let tabEls = $state<Record<string, HTMLDivElement>>({});

  /** ARIA tabs pattern: arrows move focus + activate; Home/End jump. */
  function handleTabsKeydown(e: KeyboardEvent) {
    // Let the tab-rename input handle arrow keys natively
    if ((e.target as HTMLElement | null)?.tagName === 'INPUT') return;
    const ids = tabsStore.tabs.map((t) => t.id);
    if (ids.length === 0) return;
    const currentIdx = ids.indexOf(tabsStore.activeTabId ?? '');
    let targetIdx = -1;
    if (e.key === 'ArrowRight') targetIdx = (currentIdx + 1) % ids.length;
    else if (e.key === 'ArrowLeft') targetIdx = (currentIdx - 1 + ids.length) % ids.length;
    else if (e.key === 'Home') targetIdx = 0;
    else if (e.key === 'End') targetIdx = ids.length - 1;
    if (targetIdx === -1) return;
    e.preventDefault();
    tabsStore.setActive(ids[targetIdx]);
    tabEls[ids[targetIdx]]?.focus();
  }

  /** Menu keyboard: arrows cycle items, Tab wraps, Escape closes. */
  function handleMenuKeydown(e: KeyboardEvent) {
    const items = Array.from(
      contextMenuEl?.querySelectorAll<HTMLButtonElement>('.context-menu-item') ?? [],
    );
    if (items.length === 0) return;
    const idx = items.indexOf(document.activeElement as HTMLButtonElement);
    let targetIdx = -1;
    if (e.key === 'ArrowDown') targetIdx = (idx + 1) % items.length;
    else if (e.key === 'ArrowUp') targetIdx = (idx - 1 + items.length) % items.length;
    else if (e.key === 'Home') targetIdx = 0;
    else if (e.key === 'End') targetIdx = items.length - 1;
    else if (e.key === 'Tab') targetIdx = e.shiftKey ? (idx - 1 + items.length) % items.length : (idx + 1) % items.length;
    if (targetIdx !== -1) {
      e.preventDefault();
      items[targetIdx]?.focus();
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      closeContextMenu();
    }
  }

  function handleContextMenu(e: MouseEvent, tabId: string) {
    contextMenuPos = { x: e.clientX, y: e.clientY };
    contextMenuTabId = tabId;
    contextMenuReturnFocus = document.activeElement as HTMLElement | null;
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
    contextMenuReturnFocus?.focus();
    contextMenuReturnFocus = null;
  }

  // Focus the first item when the menu opens
  $effect(() => {
    if (showContextMenu) {
      const first = contextMenuEl?.querySelector<HTMLButtonElement>('.context-menu-item');
      first?.focus();
    }
  });

  function handleNewTab() {
    tabsStore.newTab();
  }

  function handleTabClick(id: string) {
    tabsStore.setActive(id);
  }

  function handleTabMiddleClick(id: string) {
    const tab = tabsStore.tabs.find(t => t.id === id);
    if (tab && tab.content !== tab.savedContent) {
      window.dispatchEvent(new CustomEvent('tab-close-request', { detail: { tabId: id } }));
    } else {
      tabsStore.forceCloseTab(id);
    }
  }

  function handleTabClose(id: string) {
    const tab = tabsStore.tabs.find(t => t.id === id);
    if (tab && tab.content !== tab.savedContent) {
      window.dispatchEvent(new CustomEvent('tab-close-request', { detail: { tabId: id } }));
    } else {
      tabsStore.forceCloseTab(id);
    }
  }

  function emit(name: string, detail?: unknown) {
    window.dispatchEvent(new CustomEvent(name, { detail }));
  }

  function closeOtherTabs() {
    tabsStore.tabs.forEach(t => {
      if (t.id === contextMenuTabId) return;
      if (t.content !== t.savedContent) {
        window.dispatchEvent(new CustomEvent('tab-close-request', { detail: { tabId: t.id } }));
      } else {
        tabsStore.forceCloseTab(t.id);
      }
    });
    closeContextMenu();
  }

  function closeAllTabs() {
    tabsStore.tabs.forEach(t => {
      if (t.content !== t.savedContent) {
        window.dispatchEvent(new CustomEvent('tab-close-request', { detail: { tabId: t.id } }));
      } else {
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

  function revealInExplorer() {
    const tab = tabsStore.tabs.find(t => t.id === contextMenuTabId);
    if (tab?.path) {
      import('@tauri-apps/plugin-shell').then(({ open }) => {
        const dir = tab.path!.replace(/[/\\][^/\\]+$/, '');
        open(dir);
      });
    }
    closeContextMenu();
  }

  // Drag and drop reorder
  let dragIdx: number | null = null;

  function handleDragStart(idx: number) {
    dragIdx = idx;
  }

  function handleDragOver(e: DragEvent, idx: number) {
    e.preventDefault();
    if (dragIdx === null || dragIdx === idx) return;
    e.dataTransfer!.dropEffect = 'move';
  }

  function handleDrop(e: DragEvent, idx: number) {
    e.preventDefault();
    if (dragIdx === null || dragIdx === idx) return;
    tabsStore.reorder(dragIdx, idx);
    dragIdx = null;
  }

  function handleDragEnd() {
    dragIdx = null;
  }
</script>

<div class="tabbar" role="tablist" aria-label={t('tabs.openFiles')}>
  <div class="tabbar-scroll" role="presentation" onkeydown={handleTabsKeydown}>
    {#each tabsStore.tabs as tab, idx (tab.id)}
      <Tab
        element={(el) => { if (el) tabEls[tab.id] = el; }}
        fileName={tab.fileName}
        isDirty={tab.content !== tab.savedContent}
        isActive={tab.id === tabsStore.activeTabId}
        tabIndex={idx}
        onclick={() => handleTabClick(tab.id)}
        onmiddleclick={() => handleTabMiddleClick(tab.id)}
        onclose={() => handleTabClose(tab.id)}
        oncontextmenu={(e) => handleContextMenu(e, tab.id)}
        onrename={(name) => tabsStore.renameTab(tab.id, name)}
        ondragstart={() => handleDragStart(idx)}
        ondragover={(e) => handleDragOver(e, idx)}
        ondrop={(e) => handleDrop(e, idx)}
        ondragend={handleDragEnd}
      />
    {/each}
  </div>
  <button class="tabbar-new" onclick={handleNewTab} title="New tab (CmdOrCtrl+N)" aria-label={t('tabs.newTab')}>
    <svg width="14" height="14" viewBox="0 0 14 14">
      <path fill="currentColor" d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    </svg>
  </button>
</div>

{#if showContextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="context-menu-overlay"
    onclick={closeContextMenu}
    onkeydown={(e) => e.key === 'Escape' && closeContextMenu()}
    role="presentation"
  >
    <div
      class="context-menu"
      bind:this={contextMenuEl}
      style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px;"
      role="menu"
      tabindex="-1"
      aria-label={t('tabs.openFiles')}
      onkeydown={handleMenuKeydown}
    >
      <button class="context-menu-item" role="menuitem" onclick={() => { emit('menu-new-tab'); closeContextMenu(); }}>
        <span>{t('tabs.new')}</span>
        <span class="shortcut">Ctrl+N</span>
      </button>
      <button class="context-menu-item" role="menuitem" onclick={() => { emit('menu-open-file'); closeContextMenu(); }}>
        <span>{t('tabs.openFile')}</span>
        <span class="shortcut">Ctrl+O</span>
      </button>
      <div class="context-menu-sep"></div>
      <button class="context-menu-item" role="menuitem" onclick={() => { handleTabClose(contextMenuTabId); closeContextMenu(); }}>
        <span>{t('tabs.close')}</span>
        <span class="shortcut">Ctrl+W</span>
      </button>
      <button class="context-menu-item" role="menuitem" onclick={closeOtherTabs}>
        {t('tabs.closeOthers')}
      </button>
      <button class="context-menu-item" role="menuitem" onclick={closeAllTabs}>
        {t('tabs.closeAll')}
      </button>
      {#if tabsStore.tabs.find(t => t.id === contextMenuTabId)?.path}
        <div class="context-menu-sep"></div>
        <button class="context-menu-item" role="menuitem" onclick={copyPath}>
          {t('tabs.copyPath')}
        </button>
        <button class="context-menu-item" role="menuitem" onclick={revealInExplorer}>
          {t('tabs.revealInExplorer')}
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
