/**
 * Discriminated union for all cross-component editor actions.
 * The frontend dispatches these via {@link dispatchEditorAction} and
 * the editor subscribes via {@link onEditorAction}.
 */
export type EditorAction =
  | { action: 'undo' }
  | { action: 'redo' }
  | { action: 'cut' }
  | { action: 'copy' }
  | { action: 'paste' }
  | { action: 'select-all' }
  | { action: 'find' }
  | { action: 'find-replace' }
  | { action: 'set-language'; language: string }
  | { action: 'go-to-line'; line: number }
  | { action: 'search-next'; query: string; caseSensitive: boolean; useRegex: boolean }
  | { action: 'search-prev'; query: string; caseSensitive: boolean; useRegex: boolean }
  | { action: 'search'; query: string; caseSensitive: boolean; useRegex: boolean }
  | { action: 'replace'; query: string; replacement: string; caseSensitive: boolean; useRegex: boolean }
  | { action: 'replace-all'; query: string; replacement: string; caseSensitive: boolean; useRegex: boolean }
  | { action: 'add-next-occurrence' }
  | { action: 'select-all-occurrences' }
  | { action: 'jump-edit-back' }
  | { action: 'jump-edit-forward' }
  | { action: 'go-to-symbol' }
  | { action: 'jump-to-symbol'; line: number };

const EVENT_NAME = 'editor-action';

export function dispatchEditorAction(action: EditorAction): void {
  window.dispatchEvent(new CustomEvent<EditorAction>(EVENT_NAME, { detail: action }));
}

export function onEditorAction(handler: (action: EditorAction) => void): () => void {
  const listener = (e: Event) => {
    const detail = (e as CustomEvent<EditorAction>).detail;
    if (detail) handler(detail);
  };
  window.addEventListener(EVENT_NAME, listener);
  return () => window.removeEventListener(EVENT_NAME, listener);
}
