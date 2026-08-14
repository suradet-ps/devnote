<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { undo, redo, selectAll } from '@codemirror/commands';
  import { openSearchPanel, selectMatches, selectNextOccurrence } from '@codemirror/search';
  import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';
  import { createEditorState, reconfigureView, reconfigureLanguage, langCompartment } from '$lib/codemirror/setup';
  import { getLanguage } from '$lib/codemirror/extensions';
  import { reconfigureIndentGuides } from '$lib/codemirror/guides';
  import { reconfigureVisibleWhitespace } from '$lib/codemirror/whitespace';
  import { onEditorAction, type EditorAction } from '$lib/editor/actions';
  import { findNextFrom, replaceAll, findAll } from '$lib/editor/search';
  import { EditHistory } from '$lib/editor/edit-history';
  import { extractSymbols } from '$lib/editor/symbols';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { editorStatus } from '$lib/stores/editor-status.svelte';

  interface Props {
    tabId: string;
    content: string;
    language: string;
    indentGuides: boolean;
    visibleWhitespace: boolean;
    onContentChange: (content: string) => void;
    onCursorUpdate: (line: number, col: number) => void;
  }

  let {
    tabId, content, language, indentGuides, visibleWhitespace,
    onContentChange, onCursorUpdate,
  }: Props = $props();

  let editorEl: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let lastTabId: string | null = null;
  let currentLanguage = '';
  let suppressNextUpdate = false;
  let pendingCursorFrame: number | null = null;
  let editHistory = new EditHistory();
  let searchQuery = '';
  let searchMatches: { from: number; to: number }[] = [];
  let searchIndex = -1;

  function jumpToMatch(targetView: EditorView, m: { from: number; to: number }) {
    targetView.dispatch({
      selection: { anchor: m.from, head: m.to },
      effects: EditorView.scrollIntoView(m.from, { y: 'center' }),
    });
    targetView.focus();
  }

  function jumpToPos(targetView: EditorView, pos: number) {
    const clamped = Math.min(Math.max(0, pos), targetView.state.doc.length);
    targetView.dispatch({
      selection: { anchor: clamped },
      effects: EditorView.scrollIntoView(clamped, { y: 'center' }),
    });
    targetView.focus();
  }

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
    destroyEditor();
    if (!editorEl) return;
    currentLanguage = lang;

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
          // Selection stats (chars + words across all ranges)
          let chars = 0;
          const parts: string[] = [];
          for (const range of view.state.selection.ranges) {
            if (range.empty) continue;
            const text = view.state.sliceDoc(range.from, range.to);
            chars += text.length;
            parts.push(text);
          }
          const words = parts.join(' ').split(/\s+/).filter(Boolean).length;
          editorStatus.__setSelection(chars, words);
        });
      },
      (view) => {
        // Record edit sites; programmatic content syncs (tab switch,
        // replace-all) are suppressed and must not pollute the history.
        if (suppressNextUpdate) return;
        editHistory.push(view.state.selection.main.head);
      },
    );

    view = new EditorView({ state, parent: editorEl });
    lastTabId = tabId;
    editHistory = new EditHistory();
    searchQuery = '';
    searchMatches = [];
    searchIndex = -1;
    editorStatus.__setSelection(0, 0);
    view.focus();
    // If the language pack is async, apply it when it resolves
    reconfigureLanguage(view, lang);
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
      case 'add-next-occurrence':
        selectNextOccurrence(view);
        view.focus();
        break;
      case 'select-all-occurrences':
        selectMatches(view);
        view.focus();
        break;
      case 'jump-edit-back': {
        const pos = editHistory.back();
        if (pos !== null) jumpToPos(view, pos);
        break;
      }
      case 'jump-edit-forward': {
        const pos = editHistory.forward();
        if (pos !== null) jumpToPos(view, pos);
        break;
      }
      case 'go-to-symbol': {
        // The language pack may still be loading (async import). Ensure it is
        // applied before extracting, otherwise the parse tree is empty.
        void (async () => {
          const langExt = getLanguage(currentLanguage);
          if (langExt instanceof Promise) {
            const resolved = await langExt;
            view?.dispatch({ effects: [langCompartment.reconfigure(resolved)] });
          }
          if (!view) return;
          const syms = extractSymbols(view.state);
          window.dispatchEvent(new CustomEvent('symbols-ready', { detail: syms }));
        })();
        break;
      }
      case 'jump-to-symbol': {
        const lineCount = view.state.doc.lines;
        const targetLine = Math.min(Math.max(1, action.line), lineCount);
        const lineObj = view.state.doc.line(targetLine);
        view.dispatch({
          selection: { anchor: lineObj.from, head: lineObj.from },
          effects: EditorView.scrollIntoView(lineObj.from, { y: 'center' }),
        });
        view.focus();
        break;
      }
      case 'find':
      case 'find-replace':
        openSearchPanel(view);
        break;
      case 'search': {
        const { query, caseSensitive, useRegex } = action;
        const opts = { caseSensitive, useRegex };
        searchQuery = query;
        const doc = view.state.doc.toString();
        searchMatches = query ? findAll(doc, query, opts) : [];
        if (searchMatches.length === 0) {
          searchIndex = -1;
          break;
        }
        const pos = view.state.selection.main.head;
        let idx = searchMatches.findIndex((m) => m.to > pos);
        if (idx === -1) idx = 0;
        searchIndex = idx;
        jumpToMatch(view, searchMatches[idx]);
        break;
      }
      case 'search-next': {
        if (searchQuery === '') break;
        if (searchMatches.length === 0) {
          const { query, caseSensitive, useRegex } = action;
          const opts = { caseSensitive, useRegex };
          searchMatches = findAll(view.state.doc.toString(), searchQuery || query, opts);
        }
        if (searchMatches.length === 0) break;
        searchIndex = (searchIndex + 1) % searchMatches.length;
        jumpToMatch(view, searchMatches[searchIndex]);
        break;
      }
      case 'search-prev': {
        if (searchQuery === '') break;
        if (searchMatches.length === 0) {
          const { query, caseSensitive, useRegex } = action;
          const opts = { caseSensitive, useRegex };
          searchMatches = findAll(view.state.doc.toString(), searchQuery || query, opts);
        }
        if (searchMatches.length === 0) break;
        searchIndex = (searchIndex - 1 + searchMatches.length) % searchMatches.length;
        jumpToMatch(view, searchMatches[searchIndex]);
        break;
      }
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
      case 'replace': {
        const { query, replacement, caseSensitive, useRegex } = action;
        const doc = view.state.doc.toString();
        const pos = view.state.selection.main.head;
        const opts = { caseSensitive, useRegex };
        const m =
          findNextFrom(doc, query, pos, opts) ?? findNextFrom(doc, query, 0, opts);
        if (m) {
          view.dispatch({
            changes: { from: m.from, to: m.to, insert: replacement },
            selection: { anchor: m.from + replacement.length },
          });
          view.focus();
        }
        break;
      }
      case 'replace-all': {
        const { query, replacement, caseSensitive, useRegex } = action;
        const res = replaceAll(view.state.doc.toString(), query, replacement, {
          caseSensitive,
          useRegex,
        });
        if (res.count > 0) {
          suppressNextUpdate = true;
          view.dispatch({
            changes: { from: 0, to: view.state.doc.length, insert: res.content },
          });
          requestAnimationFrame(() => {
            suppressNextUpdate = false;
          });
        }
        break;
      }
    }
  }

  $effect(() => {
    if (view && tabId !== lastTabId) {
      createEditor(content, language);
    }
  });

  $effect(() => {
    if (view && content !== view.state.doc.toString()) {
      suppressNextUpdate = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
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
    void settingsStore.tabSize;
    if (view) {
      reconfigureView(view, settingsStore.settings, getTheme());
    }
  });

  $effect(() => {
    if (view) {
      reconfigureIndentGuides(view, indentGuides, settingsStore.tabSize);
      reconfigureVisibleWhitespace(view, visibleWhitespace);
    }
  });

  let removeActionListener: (() => void) | null = null;

  onMount(() => {
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

  .editor-wrapper :global(.cm-ws-space::before) {
    content: '·';
    color: var(--muted-soft);
    opacity: 0.45;
  }

  .editor-wrapper :global(.cm-ws-tab::before) {
    content: '→';
    color: var(--muted-soft);
    opacity: 0.45;
  }
</style>
