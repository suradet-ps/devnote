/**
 * Ephemeral editor-derived status for the status bar (selection size).
 * Not part of the tab model — selection is transient UI state.
 */
let _selectionChars = $state(0);
let _selectionWords = $state(0);

export const editorStatus = {
  get selectionChars() {
    return _selectionChars;
  },
  get selectionWords() {
    return _selectionWords;
  },
  /** @internal */
  __setSelection(chars: number, words: number): void {
    _selectionChars = chars;
    _selectionWords = words;
  },
};
