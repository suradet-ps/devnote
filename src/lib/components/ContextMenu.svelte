<script lang="ts">
  let { x = 0, y = 0, show = false, items = [], onClose }: {
    x: number;
    y: number;
    show: boolean;
    items: Array<{ label: string; action: () => void }>;
    onClose: () => void;
  } = $props();
</script>

{#if show}
  <div class="context-overlay" onclick={onClose} onkeydown={(e) => e.key === 'Escape' && onClose()} role="button" tabindex="-1">
    <div class="context-menu" style="left: {x}px; top: {y}px;" onclick={(e) => e.stopPropagation()} onkeydown={() => {}} role="menu" tabindex="-1">
      {#each items as item}
        <button class="context-item" role="menuitem" onclick={() => { item.action(); onClose(); }}>
          {item.label}
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .context-overlay {
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
    min-width: 160px;
    box-shadow: 0 4px 16px rgba(20, 20, 19, 0.15);
    z-index: 201;
  }

  .context-item {
    display: block;
    width: 100%;
    padding: var(--sp-xs) var(--sp-md);
    font-size: 13px;
    color: var(--ink);
    text-align: left;
    transition: background 0.1s;
  }

  .context-item:hover {
    background: var(--surface-soft);
  }
</style>
