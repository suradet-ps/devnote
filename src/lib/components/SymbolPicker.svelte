<script lang="ts">
  import { tick } from 'svelte';
  import type { SymbolInfo } from '$lib/editor/symbols';
  import { t } from '$lib/i18n/i18n.svelte';

  let { open, symbols, onSelect, onClose }: {
    open: boolean;
    symbols: SymbolInfo[];
    onSelect: (s: SymbolInfo) => void;
    onClose: () => void;
  } = $props();

  let query = $state('');
  let activeIdx = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLDivElement | null>(null);

  const filtered = $derived(
    query.trim().length > 0
      ? symbols.filter((s) => s.name.toLowerCase().includes(query.trim().toLowerCase()))
      : symbols,
  );

  $effect(() => {
    if (!open) return;
    query = '';
    activeIdx = 0;
    void (async () => {
      await tick();
      inputEl?.focus();
    })();
  });

  // Keep the active item in view when navigating with arrows
  $effect(() => {
    if (!open) return;
    listEl
      ?.querySelectorAll<HTMLElement>('.symbol-item')
      [activeIdx]?.scrollIntoView({ block: 'nearest' });
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIdx = Math.min(activeIdx + 1, filtered.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIdx = Math.max(activeIdx - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const s = filtered[activeIdx];
      if (s) onSelect(s);
    }
  }
</script>

{#if open}
  <div
    class="symbol-picker"
    role="dialog"
    tabindex="-1"
    aria-label={t('symbols.title')}
    onkeydown={handleKeydown}
  >
    <input
      bind:this={inputEl}
      class="symbol-input"
      type="text"
      placeholder={t('symbols.placeholder')}
      bind:value={query}
      aria-label={t('symbols.placeholder')}
      role="combobox"
      aria-expanded="true"
      aria-controls="symbol-list"
      aria-activedescendant={filtered[activeIdx] ? `symbol-item-${activeIdx}` : undefined}
    />
    {#if filtered.length === 0}
      <p class="symbol-empty">{t('symbols.empty')}</p>
    {:else}
      <div class="symbol-list" id="symbol-list" role="listbox" bind:this={listEl}>
        {#each filtered as s, i (s.pos)}
          <button
            class="symbol-item"
            class:active={i === activeIdx}
            id={`symbol-item-${i}`}
            role="option"
            aria-selected={i === activeIdx}
            onclick={() => onSelect(s)}
            onmouseenter={() => { activeIdx = i; }}
          >
            <span class="symbol-name">{s.name}</span>
            <span class="symbol-line">Ln {s.line}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .symbol-picker {
    position: absolute;
    top: var(--sp-xs);
    left: 50%;
    transform: translateX(-50%);
    width: 420px;
    max-width: 90%;
    background: var(--surface-card);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    padding: var(--sp-sm);
    z-index: 60;
    box-shadow: 0 8px 32px rgba(20, 20, 19, 0.15);
  }

  .symbol-input {
    width: 100%;
    height: 32px;
    padding: 0 var(--sp-xs);
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    font-size: 13px;
    color: var(--ink);
  }

  .symbol-input:focus {
    border-color: var(--primary);
  }

  .symbol-list {
    margin-top: var(--sp-xs);
    max-height: 280px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .symbol-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-sm);
    padding: 5px var(--sp-xs);
    font-size: 13px;
    color: var(--body);
    text-align: left;
    border-radius: var(--r-xs);
  }

  .symbol-item:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }

  .symbol-item.active {
    background: var(--surface-cream-strong);
    color: var(--ink);
  }

  .symbol-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .symbol-line {
    font-size: 11px;
    color: var(--muted-soft);
    flex-shrink: 0;
  }

  .symbol-empty {
    padding: var(--sp-sm);
    font-size: 13px;
    color: var(--muted-soft);
    text-align: center;
  }
</style>
