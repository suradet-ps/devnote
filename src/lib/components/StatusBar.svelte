<script lang="ts">
  import { tabsStore } from '$lib/stores/tabs';

  let line = $derived(tabsStore.activeTab?.cursorLine ?? 1);
  let col = $derived(tabsStore.activeTab?.cursorCol ?? 1);
  let content = $derived(tabsStore.activeTab?.content ?? '');
  let wordCount = $derived(content ? content.split(/\s+/).filter(Boolean).length : 0);
  let charCount = $derived(content.length);
  let language = $derived(tabsStore.activeTab?.language ?? 'text');

  const langDisplay: Record<string, string> = {
    javascript: 'JavaScript',
    typescript: 'TypeScript',
    rust: 'Rust',
    python: 'Python',
    html: 'HTML',
    css: 'CSS',
    json: 'JSON',
    markdown: 'Markdown',
    sql: 'SQL',
    xml: 'XML',
    text: 'Plain Text',
  };
</script>

<div class="statusbar">
  <div class="statusbar-left">
    <span class="statusbar-badge">{langDisplay[language] ?? language}</span>
  </div>
  <div class="statusbar-center">
    <span class="statusbar-item">UTF-8</span>
    <span class="statusbar-sep">|</span>
    <span class="statusbar-item">LF</span>
  </div>
  <div class="statusbar-right">
    <span class="statusbar-item">Ln {line}, Col {col}</span>
    <span class="statusbar-sep">|</span>
    <span class="statusbar-item">{wordCount} words</span>
    <span class="statusbar-sep">|</span>
    <span class="statusbar-item">{charCount} chars</span>
  </div>
</div>

<style>
  .statusbar {
    height: 24px;
    background: var(--surface-dark-elevated);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--sp-sm);
    font-size: 11px;
    color: var(--on-dark-soft);
    flex-shrink: 0;
    user-select: none;
  }

  .statusbar-left,
  .statusbar-center,
  .statusbar-right {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
  }

  .statusbar-badge {
    background: var(--surface-dark-soft);
    padding: 1px var(--sp-xs);
    border-radius: var(--r-xs);
    color: var(--accent-teal);
    font-size: 11px;
    font-weight: 500;
  }

  .statusbar-item {
    color: var(--on-dark-soft);
  }

  .statusbar-sep {
    color: var(--on-dark-soft);
    opacity: 0.4;
  }
</style>
