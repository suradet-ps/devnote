<script lang="ts">
  import { tabsStore } from '$lib/stores/tabs';

  let { show = false, onClose, onFind, onReplace }: {
    show: boolean;
    onClose: () => void;
    onFind: (query: string) => void;
    onReplace: (query: string, replacement: string) => void;
  } = $props();

  let query = $state('');
  let replacement = $state('');
  let isReplaceMode = $state(false);
  let caseSensitive = $state(false);
  let useRegex = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  function handleFind() {
    if (query) onFind(query);
  }

  function handleReplace() {
    if (query) onReplace(query, replacement);
  }
</script>

{#if show}
  <div class="find-panel" onkeydown={handleKeydown} role="search">
    <div class="find-row">
      <input
        class="find-input"
        type="text"
        placeholder="Find"
        bind:value={query}
        oninput={handleFind}
      />
      <button class="find-btn" onclick={() => { isReplaceMode = !isReplaceMode; }}>
        {isReplaceMode ? '▼' : '▶'}
      </button>
      <button class="find-btn" onclick={onClose}>✕</button>
    </div>
    {#if isReplaceMode}
      <div class="find-row">
        <input
          class="find-input"
          type="text"
          placeholder="Replace"
          bind:value={replacement}
        />
        <button class="find-btn find-btn-action" onclick={handleReplace}>Replace</button>
      </div>
    {/if}
    <div class="find-options">
      <button
        class="find-option"
        class:active={caseSensitive}
        onclick={() => caseSensitive = !caseSensitive}
      >Aa</button>
      <button
        class="find-option"
        class:active={useRegex}
        onclick={() => useRegex = !useRegex}
      >.*</button>
    </div>
  </div>
{/if}

<style>
  .find-panel {
    position: absolute;
    top: var(--sp-xs);
    right: var(--sp-md);
    background: var(--surface-card);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    padding: var(--sp-sm);
    min-width: 320px;
    z-index: 50;
    box-shadow: 0 4px 16px rgba(20, 20, 19, 0.1);
  }

  .find-row {
    display: flex;
    gap: var(--sp-xxs);
    margin-bottom: var(--sp-xxs);
  }

  .find-input {
    flex: 1;
    height: 32px;
    padding: 0 var(--sp-xs);
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    font-size: 13px;
    color: var(--ink);
  }

  .find-input:focus {
    border-color: var(--primary);
    outline: none;
  }

  .find-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-sm);
    color: var(--muted);
    font-size: 12px;
    transition: background 0.15s;
  }

  .find-btn:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }

  .find-btn-action {
    width: auto;
    padding: 0 var(--sp-sm);
    font-size: 12px;
    background: var(--primary);
    color: var(--on-primary);
  }

  .find-btn-action:hover {
    background: var(--primary-active);
    color: var(--on-primary);
  }

  .find-options {
    display: flex;
    gap: var(--sp-xxs);
    margin-top: var(--sp-xxs);
  }

  .find-option {
    width: 28px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-xs);
    font-size: 11px;
    color: var(--muted);
    font-weight: 500;
    transition: background 0.15s;
  }

  .find-option:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }

  .find-option.active {
    background: var(--primary);
    color: var(--on-primary);
  }
</style>
