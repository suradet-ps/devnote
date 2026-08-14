<script lang="ts">
  import { tick } from 'svelte';
  import { t } from '$lib/i18n/i18n.svelte';

  const ENCODINGS = ['UTF-8', 'UTF-16LE', 'UTF-16BE', 'windows-1252'] as const;

  let { open, fileName, detected, onSelect, onClose }: {
    open: boolean;
    fileName: string;
    detected: string;
    onSelect: (encoding: string) => void;
    onClose: () => void;
  } = $props();

  let activeIdx = $state(0);

  $effect(() => {
    if (!open) return;
    // Preselect the detected encoding if it is one of the options
    const detectedIdx = ENCODINGS.indexOf(detected as (typeof ENCODINGS)[number]);
    activeIdx = detectedIdx >= 0 ? detectedIdx : 0;
    void (async () => {
      await tick();
      document
        .querySelectorAll<HTMLElement>('.enc-option')
        [activeIdx]?.focus();
    })();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIdx = Math.min(activeIdx + 1, ENCODINGS.length - 1);
      document.querySelectorAll<HTMLElement>('.enc-option')[activeIdx]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIdx = Math.max(activeIdx - 1, 0);
      document.querySelectorAll<HTMLElement>('.enc-option')[activeIdx]?.focus();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      onSelect(ENCODINGS[activeIdx]);
    }
  }
</script>

{#if open}
  <div class="enc-picker" role="alertdialog" aria-modal="true" aria-label={t('encoding.title')} tabindex="-1" onkeydown={handleKeydown}>
    <h3 class="enc-title">{t('encoding.title')}</h3>
    <p class="enc-body">{t('encoding.body', { name: fileName, detected })}</p>
    <div class="enc-options" role="listbox" aria-label={t('encoding.title')}>
      {#each ENCODINGS as enc, i (enc)}
        <button
          class="enc-option"
          class:active={i === activeIdx}
          role="option"
          aria-selected={i === activeIdx}
          onclick={() => onSelect(enc)}
          onmouseenter={() => { activeIdx = i; }}
        >
          {enc}
          {#if enc === detected}<span class="enc-detected">({t('encoding.detectedLabel')})</span>{/if}
        </button>
      {/each}
    </div>
    <div class="enc-actions">
      <button class="enc-cancel" onclick={onClose}>{t('dialog.cancel')}</button>
    </div>
  </div>
{/if}

<style>
  .enc-picker {
    position: fixed;
    inset: 0;
    margin: auto;
    width: 380px;
    max-width: 90%;
    height: fit-content;
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    padding: var(--sp-lg);
    z-index: 600;
    box-shadow: 0 8px 32px rgba(20, 20, 19, 0.25);
  }

  .enc-title {
    font-size: 16px;
    font-weight: 500;
    color: var(--ink);
    margin-bottom: var(--sp-xs);
  }

  .enc-body {
    font-size: 13px;
    color: var(--body);
    line-height: 1.5;
    margin-bottom: var(--sp-md);
  }

  .enc-options {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: var(--sp-md);
  }

  .enc-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-xs) var(--sp-sm);
    font-size: 13px;
    font-family: 'JetBrains Mono', monospace;
    color: var(--body);
    text-align: left;
    border-radius: var(--r-sm);
  }

  .enc-option:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }

  .enc-option.active {
    background: var(--surface-cream-strong);
    color: var(--ink);
    font-weight: 500;
  }

  .enc-detected {
    font-size: 11px;
    font-family: 'Inter', sans-serif;
    color: var(--muted-soft);
  }

  .enc-actions {
    display: flex;
    justify-content: flex-end;
  }

  .enc-cancel {
    padding: var(--sp-xs) var(--sp-md);
    border-radius: var(--r-md);
    font-size: 13px;
    font-weight: 500;
    color: var(--body);
    height: 36px;
  }

  .enc-cancel:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }
</style>
