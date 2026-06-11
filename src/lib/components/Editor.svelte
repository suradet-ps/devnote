<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { createEditorState } from '$lib/codemirror/setup';
  import { tabsStore } from '$lib/stores/tabs';
  import { settingsStore } from '$lib/stores/settings';

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
  let isUpdatingFromOutside = false;

  function getTheme(): 'light' | 'dark' {
    return settingsStore.theme === 'dark' ? 'dark' : 'light';
  }

  function destroyEditor() {
    if (view) {
      view.destroy();
      view = null;
    }
  }

  function createEditor() {
    destroyEditor();
    if (!editorEl) return;

    const state = createEditorState(
      content,
      settingsStore.settings,
      getTheme(),
      language,
      (value) => {
        if (!isUpdatingFromOutside) {
          onContentChange(value);
        }
      },
      (view) => {
        const pos = view.state.selection.main.head;
        const line = view.state.doc.lineAt(pos);
        onCursorUpdate(line.number, pos - line.from + 1);
      },
    );

    view = new EditorView({ state, parent: editorEl });
  }

  onMount(() => {
    createEditor();
  });

  onDestroy(() => {
    destroyEditor();
  });

  $effect(() => {
    if (view && content !== undefined) {
      const currentContent = view.state.doc.toString();
      if (currentContent !== content) {
        isUpdatingFromOutside = true;
        view.dispatch({
          changes: { from: 0, to: currentContent.length, insert: content }
        });
        isUpdatingFromOutside = false;
      }
    }
  });

  $effect(() => {
    if (view && language) {
      createEditor();
    }
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
  }

  .editor-wrapper :global(.cm-scroller) {
    overflow: auto;
  }
</style>
