<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { undo, redo, selectAll } from '@codemirror/commands';
  import { openSearchPanel } from '@codemirror/search';
  import { createEditorState, reconfigureView } from '$lib/codemirror/setup';
  import { settingsStore } from '$lib/stores/settings.svelte';

  interface Props {
    tabId: string;
    content: string;
    language: string;
    onContentChange: (content: string) => void;
    onCursorUpdate: (line: number, col: number) => void;
  }

  let { tabId, content, language, onContentChange, onCursorUpdate }: Props = $props();

  let editorEl: HTMLDivElement;
  let view: EditorView | null = null;
  let lastTabId = '';
  let suppressNextUpdate = false;

  function getTheme(): 'light' | 'dark' {
    return settingsStore.getEffectiveTheme();
  }

  function destroyEditor() {
    if (view) {
      view.destroy();
      view = null;
    }
  }

  function createEditor(doc: string, lang: string) {
    destroyEditor();
    if (!editorEl) return;

    const state = createEditorState(
      doc,
      settingsStore.settings,
      getTheme(),
      lang,
      (value) => {
        if (!suppressNextUpdate) {
          onContentChange(value);
        }
      },
      (update) => {
        const pos = update.state.selection.main.head;
        const line = update.state.doc.lineAt(pos);
        onCursorUpdate(line.number, pos - line.from + 1);
      },
    );

    view = new EditorView({ state, parent: editorEl });
    view.focus();
  }

  // Expose editor methods to parent via custom events
  function handleEditorAction(e: Event) {
    if (!view) return;
    const detail = (e as CustomEvent).detail;
    const action = detail.action;

    switch (action) {
      case 'undo':
        undo(view);
        break;
      case 'redo':
        redo(view);
        break;
      case 'cut':
        document.execCommand('cut');
        break;
      case 'copy':
        document.execCommand('copy');
        break;
      case 'paste':
        document.execCommand('paste');
        break;
      case 'select-all':
        selectAll(view);
        break;
      case 'find':
        openSearchPanel(view);
        break;
      case 'find-replace':
        openSearchPanel(view);
        break;
      case 'go-to-line': {
        const line = detail.line;
        if (line && line > 0) {
          const lineCount = view.state.doc.lines;
          const targetLine = Math.min(Math.max(1, line), lineCount);
          const lineObj = view.state.doc.line(targetLine);
          view.dispatch({
            selection: { anchor: lineObj.from, head: lineObj.from },
            effects: EditorView.scrollIntoView(lineObj.from, { y: 'center' }),
          });
          view.focus();
        }
        break;
      }
    }
  }

  $effect(() => {
    if (tabId !== lastTabId && view) {
      lastTabId = tabId;
      createEditor(content, language);
    }
  });

  // Handle content prop changes from outside (e.g., tab switch restore)
  $effect(() => {
    if (view && content !== view.state.doc.toString()) {
      suppressNextUpdate = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
      });
      requestAnimationFrame(() => { suppressNextUpdate = false; });
    }
  });

  // Handle font size changes
  $effect(() => {
    const size = settingsStore.fontSize;
    const wrap = settingsStore.wordWrap;
    const theme = getTheme();
    if (view) {
      reconfigureView(view, settingsStore.settings, theme);
    }
  });

  onMount(() => {
    createEditor(content, language);
    lastTabId = tabId;

    // Listen for editor actions
    window.addEventListener('editor-action', handleEditorAction as EventListener);
  });

  onDestroy(() => {
    destroyEditor();
    window.removeEventListener('editor-action', handleEditorAction as EventListener);
  });
</script>

<div class="editor-wrapper" bind:this={editorEl}></div>

<style>
  .editor-wrapper {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .editor-wrapper :global(.cm-editor) {
    height: 100%;
    flex: 1;
    outline: none;
  }

  .editor-wrapper :global(.cm-scroller) {
    overflow: auto;
    font-family: 'JetBrains Mono', monospace;
  }
</style>
