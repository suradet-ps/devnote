<script lang="ts">
  import { tick } from 'svelte';
  import { dispatchEditorAction } from '$lib/editor/actions';

  let { show = false, onClose }: {
    show: boolean;
    onClose: () => void;
  } = $props();

  let query = $state('');
  let replacement = $state('');
  let replaceMode = $state(false);
  let caseSensitive = $state(false);
  let useRegex = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      dispatchEditorAction({ action: 'search-next', query, caseSensitive, useRegex });
    } else if (e.key === 'Enter' && e.shiftKey) {
      e.preventDefault();
      dispatchEditorAction({ action: 'search-prev', query, caseSensitive, useRegex });
    }
  }

  $effect(() => {
    if (show) {
      void (async () => {
        await tick();
        const input = document.querySelector<HTMLInputElement>('.find-input');
        input?.focus();
        input?.select();
      })();
    }
  });
</script>

{#if show}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="find-panel" onkeydown={handleKeydown} role="search" aria-label="Find and Replace">
    <div class="find-row">
      <input
        class="find-input"
        type="text"
        placeholder="Find"
        bind:value={query}
        aria-label="Find text"
        oninput={() => {
          dispatchEditorAction({ action: 'search', query, caseSensitive, useRegex });
        }}
      />
      <button class="find-btn" onclick={() => {
        dispatchEditorAction({ action: 'search-prev', query, caseSensitive, useRegex });
      }} aria-label="Previous match" title="Previous match (Shift+Enter)">↑</button>
      <button class="find-btn" onclick={() => {
        dispatchEditorAction({ action: 'search-next', query, caseSensitive, useRegex });
      }} aria-label="Next match" title="Next match (Enter)">↓</button>
      <button class="find-btn" onclick={() => { replaceMode = !replaceMode; }} aria-label="Toggle replace" title="Toggle Replace">
        {replaceMode ? '▼' : '▶'}
      </button>
      <button class="find-btn" onclick={onClose} aria-label="Close find panel">✕</button>
    </div>
    {#if replaceMode}
      <div class="find-row">
        <input
          class="find-input"
          type="text"
          placeholder="Replace"
          bind:value={replacement}
          aria-label="Replace with"
          onkeydown={(e: KeyboardEvent) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              dispatchEditorAction({ action: 'replace', query, replacement, caseSensitive, useRegex });
            }
          }}
        />
        <button
          class="find-btn find-btn-action"
          onclick={() => {
            dispatchEditorAction({ action: 'replace', query, replacement, caseSensitive, useRegex });
          }}
          aria-label="Replace current match"
        >Replace</button>
        <button
          class="find-btn find-btn-action"
          onclick={() => {
            dispatchEditorAction({ action: 'replace-all', query, replacement, caseSensitive, useRegex });
          }}
          aria-label="Replace all matches"
        >All</button>
      </div>
    {/if}
    <div class="find-options">
      <button
        class="find-option"
        class:active={caseSensitive}
        onclick={() => { caseSensitive = !caseSensitive; }}
        title="Match Case"
        aria-label="Match case"
        aria-pressed={caseSensitive}
      >Aa</button>
      <button
        class="find-option"
        class:active={useRegex}
        onclick={() => { useRegex = !useRegex; }}
        title="Use Regular Expression"
        aria-label="Use regular expression"
        aria-pressed={useRegex}
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
    min-width: 360px;
    z-index: 50;
    box-shadow: 0 4px 16px rgba(20, 20, 19, 0.1);
  }

  .find-row {
    display: flex;
    gap: var(--sp-xxs);
    margin-bottom: var(--sp-xxs);
    align-items: center;
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
    flex-shrink: 0;
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
