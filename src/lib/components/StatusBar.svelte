<script lang="ts">
  import { tabsStore } from '$lib/stores/tabs.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';

  let line = $derived(tabsStore.activeTab?.cursorLine ?? 1);
  let col = $derived(tabsStore.activeTab?.cursorCol ?? 1);
  let content = $derived(tabsStore.activeTab?.content ?? '');
  let wordCount = $derived(content ? content.split(/\s+/).filter(Boolean).length : 0);
  let charCount = $derived(content.length);
  let language = $derived(tabsStore.activeTab?.language ?? 'text');
  let encoding = $derived(tabsStore.activeTab?.encoding ?? 'UTF-8');
  let lineEnding = $derived(tabsStore.activeTab?.lineEnding ?? 'LF');
  let fileSize = $derived(content ? content.length : 0);

  let showLangPicker = $state(false);

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
    vue: 'Vue',
    bash: 'Shell',
    go: 'Go',
    ruby: 'Ruby',
    cpp: 'C/C++',
    java: 'Java',
    php: 'PHP',
    yaml: 'YAML',
    toml: 'TOML',
    text: 'Plain Text',
  };

  const languages = [
    { key: 'text', label: 'Plain Text' },
    { key: 'javascript', label: 'JavaScript' },
    { key: 'typescript', label: 'TypeScript' },
    { key: 'rust', label: 'Rust' },
    { key: 'python', label: 'Python' },
    { key: 'html', label: 'HTML' },
    { key: 'css', label: 'CSS' },
    { key: 'json', label: 'JSON' },
    { key: 'markdown', label: 'Markdown' },
    { key: 'sql', label: 'SQL' },
    { key: 'xml', label: 'XML' },
    { key: 'yaml', label: 'YAML' },
    { key: 'toml', label: 'TOML' },
    { key: 'go', label: 'Go' },
    { key: 'ruby', label: 'Ruby' },
    { key: 'cpp', label: 'C/C++' },
    { key: 'java', label: 'Java' },
    { key: 'php', label: 'PHP' },
    { key: 'bash', label: 'Shell' },
  ];

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function handleLangSelect(key: string) {
    const tab = tabsStore.activeTab;
    if (tab) {
      tabsStore.setLanguage(tab.id, key);
      showLangPicker = false;
    }
  }
</script>

{#if settingsStore.showStatusBar}
  <div class="statusbar" role="status" aria-label="Status bar">
    <div class="statusbar-left">
      <button
        class="statusbar-badge"
        onclick={() => showLangPicker = !showLangPicker}
        onblur={() => setTimeout(() => showLangPicker = false, 200)}
        aria-label="Select language"
        aria-haspopup="listbox"
        aria-expanded={showLangPicker}
      >
        {langDisplay[language] ?? language}
      </button>
      {#if showLangPicker}
        <div class="lang-picker" role="listbox">
          {#each languages as lang}
            <button
              class="lang-option"
              class:active={language === lang.key}
              role="option"
              aria-selected={language === lang.key}
              onclick={() => handleLangSelect(lang.key)}
            >
              {lang.label}
            </button>
          {/each}
        </div>
      {/if}
      {#if settingsStore.wordWrap}
        <span class="statusbar-item statusbar-tag">Wrap</span>
      {/if}
      {#if fileSize > 102400}
        <span class="statusbar-item">{formatFileSize(fileSize)}</span>
      {/if}
    </div>
    <div class="statusbar-center">
      <span class="statusbar-item">{encoding}</span>
      <span class="statusbar-sep">·</span>
      <span class="statusbar-item">{lineEnding}</span>
    </div>
    <div class="statusbar-right">
      <span class="statusbar-item">Ln {line}, Col {col}</span>
      <span class="statusbar-sep">·</span>
      <span class="statusbar-item">{wordCount} words</span>
      <span class="statusbar-sep">·</span>
      <span class="statusbar-item">{charCount} chars</span>
    </div>
  </div>
{/if}

<style>
  .statusbar {
    height: 24px;
    background: var(--surface-soft);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--sp-sm);
    font-size: 11px;
    color: var(--muted);
    flex-shrink: 0;
    user-select: none;
    border-top: 1px solid var(--hairline);
    position: relative;
  }

  .statusbar-left,
  .statusbar-center,
  .statusbar-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .statusbar-left {
    position: relative;
  }

  .statusbar-badge {
    background: var(--surface-card);
    padding: 1px 6px;
    border-radius: var(--r-xs);
    color: var(--primary);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
  }

  .statusbar-badge:hover {
    background: var(--surface-cream-strong);
  }

  .statusbar-item {
    color: var(--muted);
  }

  .statusbar-tag {
    color: var(--accent-amber);
    font-weight: 500;
  }

  .statusbar-sep {
    color: var(--hairline);
  }

  .lang-picker {
    position: absolute;
    bottom: 28px;
    left: 0;
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-md);
    padding: var(--sp-xxs) 0;
    min-width: 150px;
    max-height: 240px;
    overflow-y: auto;
    box-shadow: 0 4px 16px rgba(20, 20, 19, 0.15);
    z-index: 300;
  }

  .lang-option {
    display: block;
    width: 100%;
    padding: 4px var(--sp-sm);
    font-size: 11px;
    color: var(--muted);
    text-align: left;
    transition: background 0.1s;
  }

  .lang-option:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }

  .lang-option.active {
    color: var(--primary);
    font-weight: 500;
  }
</style>
