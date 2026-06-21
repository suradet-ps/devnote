import { Compartment, EditorState, type Extension } from '@codemirror/state';
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLineGutter,
  highlightSpecialChars,
  drawSelection,
  dropCursor,
  rectangularSelection,
  crosshairCursor,
  highlightActiveLine,
} from '@codemirror/view';
import {
  history,
  defaultKeymap,
  historyKeymap,
  indentWithTab,
} from '@codemirror/commands';
import {
  bracketMatching,
  indentOnInput,
  foldGutter,
  foldKeymap,
  syntaxHighlighting,
  defaultHighlightStyle,
} from '@codemirror/language';
import {
  closeBrackets,
  closeBracketsKeymap,
  autocompletion,
  completionKeymap,
} from '@codemirror/autocomplete';
import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
import { lintKeymap } from '@codemirror/lint';
import { oneDark } from '@codemirror/theme-one-dark';
import { devnotesLightTheme } from './theme';
import { getLanguage } from './extensions';
import type { Settings } from '$lib/stores/settings.svelte';

export const wrapCompartment = new Compartment();
export const fontSizeCompartment = new Compartment();
export const themeCompartment = new Compartment();
export const langCompartment = new Compartment();

export function createEditorState(
  content: string,
  settings: Settings,
  theme: 'light' | 'dark',
  language: string,
  onChange: (value: string) => void,
  onSelectionChange: (view: EditorView) => void,
): EditorState {
  const langExtension = getLanguage(language);

  // If the language is async-loaded, start with an empty compartment
  // and let `reconfigureLanguage` apply the resolved extension.
  const initialLang: Extension =
    langExtension instanceof Promise ? [] : (langExtension as unknown as Extension);

  return EditorState.create({
    doc: content,
    extensions: [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      history(),
      foldGutter(),
      drawSelection(),
      dropCursor(),
      EditorState.allowMultipleSelections.of(true),
      indentOnInput(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      bracketMatching(),
      closeBrackets(),
      autocompletion(),
      rectangularSelection(),
      crosshairCursor(),
      highlightActiveLine(),
      highlightSelectionMatches(),
      keymap.of([
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...searchKeymap,
        ...historyKeymap,
        ...foldKeymap,
        ...completionKeymap,
        ...lintKeymap,
        indentWithTab,
      ]),
      wrapCompartment.of(settings.wordWrap ? EditorView.lineWrapping : []),
      themeCompartment.of(theme === 'dark' ? oneDark : devnotesLightTheme),
      fontSizeCompartment.of(
        EditorView.theme({
          '&': { fontSize: `${settings.fontSize}px` },
          '.cm-scroller': { fontFamily: `'${settings.fontFamily}', monospace` },
        }),
      ),
      langCompartment.of(initialLang),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          onChange(update.state.doc.toString());
        }
        if (update.selectionSet) {
          onSelectionChange(update.view);
        }
      }),
    ],
  });
}

export function reconfigureView(
  view: EditorView,
  settings: Settings,
  theme: 'light' | 'dark',
): void {
  view.dispatch({
    effects: [
      wrapCompartment.reconfigure(settings.wordWrap ? EditorView.lineWrapping : []),
      themeCompartment.reconfigure(theme === 'dark' ? oneDark : devnotesLightTheme),
      fontSizeCompartment.reconfigure(
        EditorView.theme({
          '&': { fontSize: `${settings.fontSize}px` },
          '.cm-scroller': { fontFamily: `'${settings.fontFamily}', monospace` },
        }),
      ),
    ],
  });
}

export function reconfigureLanguage(view: EditorView, language: string): void {
  const langExtension = getLanguage(language);
  if (langExtension instanceof Promise) {
    // Initial empty reconfigure; resolved extension is applied when ready.
    // (CodeMirror's Compartment reconfigure cannot be called with a Promise.)
    langExtension.then((resolved) => {
      view.dispatch({
        effects: [langCompartment.reconfigure(resolved as unknown as Extension)],
      });
    });
  } else {
    view.dispatch({
      effects: [langCompartment.reconfigure(langExtension as unknown as Extension)],
    });
  }
}
