<script lang="ts">
  interface Props {
    fileName: string;
    isDirty: boolean;
    isActive: boolean;
    onclick: () => void;
    onclose: () => void;
    oncontextmenu: (e: MouseEvent) => void;
  }

  let { fileName, isDirty, isActive, onclick, onclose, oncontextmenu }: Props = $props();
</script>

<div
  class="tab"
  class:active={isActive}
  {onclick}
  oncontextmenu={(e) => { e.preventDefault(); oncontextmenu(e); }}
  role="tab"
  tabindex="-1"
>
  <span class="tab-name">{fileName}</span>
  {#if isDirty}
    <span class="tab-dot">•</span>
  {/if}
  <button
    class="tab-close"
    onclick={(e) => { e.stopPropagation(); onclose(); }}
    title="Close"
  >
    <svg width="10" height="10" viewBox="0 0 10 10">
      <path fill="currentColor" d="M2.8 1.8L5 4l2.2-2.2.7.7L5.7 4.7l2.2 2.2-.7.7L5 5.4l-2.2 2.2-.7-.7L4.3 4.7 2.1 2.5z"/>
    </svg>
  </button>
</div>

<style>
  .tab {
    height: 36px;
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xxs);
    padding: 0 var(--sp-sm);
    background: var(--surface-dark-soft);
    color: var(--on-dark-soft);
    font-size: 12px;
    border-radius: var(--r-sm) var(--r-sm) 0 0;
    cursor: pointer;
    white-space: nowrap;
    max-width: 180px;
    min-width: 80px;
    flex-shrink: 0;
    transition: background 0.15s, color 0.15s;
    position: relative;
  }

  .tab:hover {
    background: var(--surface-dark-elevated);
    color: var(--on-dark);
  }

  .tab.active {
    background: var(--canvas);
    color: var(--ink);
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

  .tab.active .tab-dot {
    color: var(--primary);
  }

  .tab-close {
    width: 18px;
    height: 18px;
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
    background: var(--surface-card);
    color: var(--ink);
  }

  .tab.active .tab-close:hover {
    background: var(--hairline);
  }
</style>
