<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  interface Props {
    open: boolean;
    title: string;
    message: string;
    onSave?: () => void;
    onDiscard?: () => void;
    onCancel?: () => void;
  }

  let { open, title, message, onSave, onDiscard, onCancel }: Props = $props();
</script>

{#if open}
  <div class="dialog-backdrop" onclick={onCancel} onkeydown={(e) => e.key === 'Escape' && onCancel?.()} role="button" tabindex="-1">
    <div class="dialog" onclick={(e) => e.stopPropagation()} onkeydown={() => {}} role="dialog" tabindex="-1">
      <h2 class="dialog-title">{title}</h2>
      <p class="dialog-message">{message}</p>
      <div class="dialog-actions">
        <button class="dialog-btn dialog-btn-save" onclick={onSave}>Save</button>
        <button class="dialog-btn dialog-btn-discard" onclick={onDiscard}>Don't Save</button>
        <button class="dialog-btn dialog-btn-cancel" onclick={onCancel}>Cancel</button>
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
  }

  .dialog {
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    padding: var(--sp-lg);
    min-width: 380px;
    max-width: 480px;
    box-shadow: 0 8px 32px rgba(20, 20, 19, 0.2);
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
