<script lang="ts">
  import { tabsStore } from '$lib/stores/tabs';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const appWindow = getCurrentWindow();

  let title = $derived.by(() => {
    const tab = tabsStore.activeTab;
    if (!tab) return 'text-rs';
    const dirty = tab.content !== tab.savedContent ? '• ' : '';
    return `${dirty}text-rs — ${tab.fileName}`;
  });

  function handleMinimize() {
    appWindow.minimize();
  }

  function handleMaximize() {
    appWindow.toggleMaximize();
  }

  async function handleClose() {
    if (tabsStore.hasDirtyTabs()) {
      window.dispatchEvent(new CustomEvent('window-close-request'));
    } else {
      appWindow.close();
    }
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="titlebar-title" data-tauri-drag-region>
    {title}
  </div>
  <div class="titlebar-controls">
    <button class="titlebar-btn" onclick={handleMinimize} title="minimize">
      <svg width="12" height="12" viewBox="0 0 12 12">
        <path fill="currentColor" d="M2 6h8v1H2z"/>
      </svg>
    </button>
    <button class="titlebar-btn" onclick={handleMaximize} title="maximize">
      <svg width="12" height="12" viewBox="0 0 12 12">
        <path fill="none" stroke="currentColor" stroke-width="1.2" d="M2 3.5h7v5.5H2z"/>
      </svg>
    </button>
    <button class="titlebar-btn titlebar-btn-close" onclick={handleClose} title="close">
      <svg width="12" height="12" viewBox="0 0 12 12">
        <path fill="currentColor" d="M3.4 2.3L6 4.9l2.6-2.6.9.9L6.9 5.8l2.6 2.6-.9.9L6 6.7l-2.6 2.7-.9-.9L5.1 5.8 2.5 3.2z"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: 32px;
    background: var(--surface-dark);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--sp-xs);
    user-select: none;
    flex-shrink: 0;
    position: relative;
    z-index: 100;
  }

  .titlebar-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--on-dark);
    padding-left: var(--sp-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .titlebar-controls {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }

  .titlebar-btn {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-sm);
    color: var(--on-dark-soft);
    transition: background 0.15s, color 0.15s;
  }

  .titlebar-btn:hover {
    background: var(--surface-dark-soft);
    color: var(--on-dark);
  }

  .titlebar-btn-close:hover {
    background: var(--error);
    color: var(--on-primary);
  }
</style>
