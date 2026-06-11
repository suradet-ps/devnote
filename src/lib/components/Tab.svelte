<script lang="ts">
  interface Props {
    fileName: string;
    isDirty: boolean;
    isActive: boolean;
    onclick: () => void;
    onclose: () => void;
    oncontextmenu: (e: MouseEvent) => void;
    onrename: (name: string) => void;
  }

  let { fileName, isDirty, isActive, onclick, onclose, oncontextmenu, onrename }: Props = $props();

  let editing = $state(false);
  let editValue = $state('');
  let inputEl: HTMLInputElement;

  function handleDblClick() {
    editValue = fileName;
    editing = true;
    setTimeout(() => {
      inputEl?.focus();
      inputEl?.select();
    }, 0);
  }

  function commitRename() {
    const name = editValue.trim();
    if (name && name !== fileName) {
      onrename(name);
    }
    editing = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      commitRename();
    } else if (e.key === 'Escape') {
      editing = false;
    }
  }
</script>

<div
  class="tab"
  class:active={isActive}
  {onclick}
  ondblclick={handleDblClick}
  oncontextmenu={(e) => { e.preventDefault(); oncontextmenu(e); }}
  onkeydown={(e) => e.key === 'Enter' && onclick()}
  role="tab"
  tabindex="-1"
>
  {#if editing}
    <input
      bind:this={inputEl}
      class="tab-rename-input"
      type="text"
      bind:value={editValue}
      onblur={commitRename}
      onkeydown={handleKeydown}
      onclick={(e) => e.stopPropagation()}
    />
  {:else}
    <span class="tab-name">{fileName}</span>
  {/if}
  {#if isDirty && !editing}
    <span class="tab-dot">•</span>
  {/if}
  <button
    class="tab-close"
    onclick={(e) => { e.stopPropagation(); onclose(); }}
    title="Close"
  >
    <svg width="8" height="8" viewBox="0 0 8 8">
      <path fill="currentColor" d="M1.8 0.8L4 3l2.2-2.2.6.6L4.6 3.6l2.2 2.2-.6.6L4 4.2l-2.2 2.2-.6-.6L3.4 3.6 1.2 1.4z"/>
    </svg>
  </button>
</div>

<style>
  .tab {
    height: 32px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 10px;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    border-radius: var(--r-sm);
    cursor: pointer;
    white-space: nowrap;
    max-width: 180px;
    min-width: 80px;
    flex-shrink: 0;
    transition: background 0.15s, color 0.15s;
    position: relative;
    margin: 3px 1px;
  }

  .tab:hover {
    background: var(--surface-card);
    color: var(--body);
  }

  .tab.active {
    background: var(--canvas);
    color: var(--ink);
    font-weight: 500;
    box-shadow: 0 1px 3px rgba(20, 20, 19, 0.06);
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab-dot {
    color: var(--primary);
    font-size: 14px;
    line-height: 1;
  }

  .tab-rename-input {
    width: 100%;
    min-width: 40px;
    height: 20px;
    padding: 0 4px;
    font-size: 12px;
    font-family: inherit;
    color: var(--ink);
    background: var(--canvas);
    border: 1px solid var(--primary);
    border-radius: var(--r-xs);
    outline: none;
  }

  .tab-close {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-xs);
    color: var(--muted-soft);
    opacity: 0;
    transition: opacity 0.15s, background 0.15s;
    flex-shrink: 0;
  }

  .tab:hover .tab-close {
    opacity: 1;
  }

  .tab-close:hover {
    background: var(--surface-cream-strong);
    color: var(--ink);
  }
</style>
