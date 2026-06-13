<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { undo, redo, selectAll } from '@codemirror/commands';
  import { openSearchPanel } from '@codemirror/search';
  import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';
  import { createEditorState, reconfigureView, reconfigureLanguage } from '$lib/codemirror/setup';
  import { onEditorAction, type EditorAction } from '$lib/editor/actions';
  import { settingsStore } from '$lib/stores/settings.svelte';

  interface Props {
    tabId: string;
    content: string;
    language: string;
    onContentChange: (content: string) => void;
    onCursorUpdate: (line: number, col: number) => void;
  }

  let { tabId, content, language, onContentChange, onCursorUpdate }: Props = $props();

  let editorEl: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let lastTabId: string | null = null;
  let suppressNextUpdate = false;
  let pendingCursorFrame: number | null = null;

  function getTheme(): 'light' | 'dark' {
    return settingsStore.getEffectiveTheme();
  }

  function destroyEditor() {
    if (pendingCursorFrame !== null) {
      cancelAnimationFrame(pendingCursorFrame);
      pendingCursorFrame = null;
    }
    if (view) {
      view.destroy();
      view = null;
    }
  }

  function createEditor(doc: string, lang: string) {
    console.log('[Editor] createEditor() called', {
      tabId,
      docLength: doc.length,
      docPreview: doc.slice(0, 80),
      lang,
      hasEditorEl: !!editorEl,
    });
    destroyEditor();
    if (!editorEl) {
      console.warn('[Editor] createEditor aborted: editorEl not bound');
      return;
    }

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
      (view) => {
        // Coalesce cursor updates to one per frame to avoid StatusBar storms
        if (pendingCursorFrame !== null) return;
        pendingCursorFrame = requestAnimationFrame(() => {
          pendingCursorFrame = null;
          const pos = view.state.selection.main.head;
          const line = view.state.doc.lineAt(pos);
          onCursorUpdate(line.number, pos - line.from + 1);
        });
      },
    );

    view = new EditorView({ state, parent: editorEl });
    lastTabId = tabId;
    view.focus();
    // If the language pack is async, apply it when it resolves
    reconfigureLanguage(view, lang);
    console.log('[Editor] view created', {
      docLengthInView: view.state.doc.length,
      docPreviewInView: view.state.doc.toString().slice(0, 80),
    });
  }

  async function handleEditorAction(action: EditorAction) {
    if (!view) return;
    switch (action.action) {
      case 'undo':
        undo(view);
        break;
      case 'redo':
        redo(view);
        break;
      case 'cut': {
        const { from, to } = view.state.selection.main;
        if (from === to) break;
        const text = view.state.sliceDoc(from, to);
        try { await writeText(text); } catch (e) { console.error('clipboard write failed', e); return; }
        view.dispatch({ changes: { from, to } });
        break;
      }
      case 'copy': {
        const { from, to } = view.state.selection.main;
        if (from === to) break;
        const text = view.state.sliceDoc(from, to);
        try { await writeText(text); } catch (e) { console.error('clipboard write failed', e); }
        break;
      }
      case 'paste': {
        let text: string;
        try { text = await readText(); } catch (e) { console.error('clipboard read failed', e); return; }
        const { from, to } = view.state.selection.main;
        view.dispatch({ changes: { from, to, insert: text } });
        break;
      }
      case 'select-all':
        selectAll(view);
        break;
      case 'find':
      case 'find-replace':
        openSearchPanel(view);
        break;
      case 'set-language': {
        if (action.language) {
          reconfigureLanguage(view, action.language);
        }
        break;
      }
      case 'go-to-line': {
        const line = action.line;
        if (line > 0) {
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
      // search/search-prev/search-next/replace/replace-all are no-ops here;
      // CodeMirror's built-in search panel handles them once openSearchPanel
      // is active. FindReplace still dispatches them for future use.
    }
  }

  $effect(() => {
    // Read all the props the effect depends on so Svelte 5's reactivity
    // tracks them. Capture into locals for the log message.
    const t = tabId;
    const l = language;
    const c = content;
    console.log('[Editor] effect#1 (tabId change) check', {
      tabId: t,
      contentLength: c.length,
      contentPreview: c.slice(0, 80),
      language: l,
      hasView: !!view,
      lastTabId,
      willRecreate: view != null && t !== lastTabId,
    });
    if (view && t !== lastTabId) {
      createEditor(c, l);
    }
  });

  $effect(() => {
    const c = content;
    if (view && c !== view.state.doc.toString()) {
      console.log('[Editor] effect#2 (content sync) dispatching', {
        contentLength: c.length,
        viewDocLength: view.state.doc.length,
      });
      suppressNextUpdate = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: c },
      });
      requestAnimationFrame(() => {
        suppressNextUpdate = false;
      });
    }
  });

  $effect(() => {
    // Read settings for reactivity
    void settingsStore.themeVersion;
    void settingsStore.fontSize;
    void settingsStore.wordWrap;
    if (view) {
      reconfigureView(view, settingsStore.settings, getTheme());
    }
  });

  let removeActionListener: (() => void) | null = null;

  onMount(() => {
    console.log('[Editor] onMount fired', {
      tabId,
      contentLength: content.length,
      contentPreview: content.slice(0, 80),
    });
    createEditor(content, language);
    removeActionListener = onEditorAction(handleEditorAction);
  });

  onDestroy(() => {
    destroyEditor();
    removeActionListener?.();
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
