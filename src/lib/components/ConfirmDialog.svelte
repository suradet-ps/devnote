<script lang="ts">
  import { tick } from 'svelte';
  import { t } from '$lib/i18n/i18n.svelte';

  interface Props {
    open: boolean;
    title: string;
    message: string;
    saveLabel?: string;
    discardLabel?: string;
    cancelLabel?: string;
    showSave?: boolean;
    showDiscard?: boolean;
    showCancel?: boolean;
    onSave?: () => void;
    onDiscard?: () => void;
    onCancel?: () => void;
  }

  let {
    open = false,
    title = '',
    message = '',
    saveLabel = t('dialog.save'),
    discardLabel = t('dialog.dontSave'),
    cancelLabel = t('dialog.cancel'),
    showSave = true,
    showDiscard = true,
    showCancel = true,
    onSave,
    onDiscard,
    onCancel,
  }: Props = $props();

  let dialogEl: HTMLDivElement | undefined = $state();
  let returnFocusEl: HTMLElement | null = null;

  // Esc closes the dialog while it is open. The previous implementation
  // attached the handler to a `role="presentation"` div which is not
  // focusable, so keydown events never reached it.
  $effect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onCancel?.();
      }
    };
    document.addEventListener('keydown', handler, { capture: true });
    return () => document.removeEventListener('keydown', handler, { capture: true });
  });

  // Focus the primary (Save) button on open for keyboard users, and remember
  // where focus came from so we can return it when the dialog closes.
  $effect(() => {
    if (!open) return;
    returnFocusEl = document.activeElement as HTMLElement | null;
    void (async () => {
      await tick();
      dialogEl
        ?.querySelector<HTMLButtonElement>('.dialog-btn')
        ?.focus();
    })();
  });

  // Focus trap: Tab / Shift+Tab cycle inside the dialog only.
  $effect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;
      const focusables = Array.from(
        dialogEl?.querySelectorAll<HTMLElement>('button') ?? [],
      );
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement;
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener('keydown', handler, { capture: true });
    return () => document.removeEventListener('keydown', handler, { capture: true });
  });

  // Return focus to the previously focused element (usually the editor).
  $effect(() => {
    if (!open && returnFocusEl) {
      const el = returnFocusEl;
      returnFocusEl = null;
      el.focus();
    }
  });
</script>

{#if open}
  <div
    class="dialog-backdrop"
    onclick={onCancel}
    role="presentation"
  >
    <div
      class="dialog"
      bind:this={dialogEl}
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      aria-describedby="dialog-message"
      tabindex="-1"
    >
      <h2 id="dialog-title" class="dialog-title">{title}</h2>
      <p id="dialog-message" class="dialog-message">{message}</p>
      <div class="dialog-actions">
        {#if showSave}
          <button class="dialog-btn dialog-btn-save" onclick={onSave}>{saveLabel}</button>
        {/if}
        {#if showDiscard}
          <button class="dialog-btn dialog-btn-discard" onclick={onDiscard}>{discardLabel}</button>
        {/if}
        {#if showCancel}
          <button class="dialog-btn dialog-btn-cancel" onclick={onCancel}>{cancelLabel}</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(20, 20, 19, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 500;
    animation: fadeIn 0.15s ease-out;
  }

  @media (prefers-reduced-motion: reduce) {
    .dialog-backdrop {
      animation: none;
    }
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .dialog {
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    padding: var(--sp-lg);
    min-width: 380px;
    max-width: 480px;
    box-shadow: 0 8px 32px rgba(20, 20, 19, 0.2);
    animation: slideUp 0.15s ease-out;
  }

  @media (prefers-reduced-motion: reduce) {
    .dialog {
      animation: none;
    }
  }

  @keyframes slideUp {
    from { transform: translateY(8px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .dialog-title {
    font-size: 16px;
    font-weight: 500;
    color: var(--ink);
    margin-bottom: var(--sp-xs);
  }

  .dialog-message {
    font-size: 13px;
    color: var(--body);
    margin-bottom: var(--sp-lg);
    line-height: 1.5;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-xs);
  }

  .dialog-btn {
    padding: var(--sp-xs) var(--sp-md);
    border-radius: var(--r-md);
    font-size: 13px;
    font-weight: 500;
    height: 36px;
    min-width: 80px;
    transition: background 0.15s;
  }

  .dialog-btn-save {
    background: var(--primary);
    color: var(--on-primary);
  }

  .dialog-btn-save:hover {
    background: var(--primary-active);
  }

  .dialog-btn-discard {
    background: var(--surface-card);
    color: var(--ink);
  }

  .dialog-btn-discard:hover {
    background: var(--surface-cream-strong);
  }

  .dialog-btn-cancel {
    background: transparent;
    color: var(--body);
  }

  .dialog-btn-cancel:hover {
    background: var(--surface-soft);
  }
</style>
