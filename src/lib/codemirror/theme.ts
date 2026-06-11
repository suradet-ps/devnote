import { EditorView } from '@codemirror/view';

export const textRsLightTheme = EditorView.theme({
  '&': {
    backgroundColor: 'var(--canvas)',
    color: 'var(--ink)',
  },
  '.cm-content': {
    caretColor: 'var(--primary)',
    fontFamily: 'var(--font-mono)',
  },
  '.cm-cursor': {
    borderLeftColor: 'var(--primary)',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
    backgroundColor: 'rgba(204, 120, 92, 0.15)',
  },
  '.cm-activeLine': {
    backgroundColor: 'rgba(204, 120, 92, 0.06)',
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'rgba(204, 120, 92, 0.10)',
  },
  '.cm-gutters': {
    backgroundColor: 'var(--surface-soft)',
    color: 'var(--muted-soft)',
    borderRight: '1px solid var(--hairline)',
  },
  '.cm-lineNumbers .cm-gutterElement': {
    color: 'var(--muted-soft)',
  },
  '.cm-foldPlaceholder': {
    backgroundColor: 'var(--surface-card)',
    color: 'var(--muted)',
  },
  '.cm-matchingBracket': {
    backgroundColor: 'rgba(204, 120, 92, 0.2)',
    outline: '1px solid var(--primary)',
  },
  '.cm-searchMatch': {
    backgroundColor: 'rgba(232, 165, 90, 0.3)',
    outline: '1px solid var(--accent-amber)',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: 'rgba(204, 120, 92, 0.35)',
  },
}, { dark: false });
