import { Compartment, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';

/**
 * Indent guides (Roadmap Phase 4): vertical guide lines at every tab-stop
 * column. CSS-only, render-only — a repeating gradient aligned to the tab
 * grid; nothing is added to the document model.
 */
export const indentGuidesCompartment = new Compartment();

export function indentGuidesExt(tabSize: number): Extension {
  const edge = `calc(${tabSize}ch - 1px)`;
  const color = 'color-mix(in srgb, var(--hairline) 55%, transparent)';
  return EditorView.contentAttributes.of({
    style: [
      'background-image: repeating-linear-gradient(to right,',
      'transparent 0,',
      `transparent ${edge},`,
      `${color} ${edge},`,
      `${color} calc(${tabSize}ch)`,
      ');',
    ].join(' '),
  });
}

/** Toggle indent guides on an existing view. */
export function reconfigureIndentGuides(
  view: EditorView,
  enabled: boolean,
  tabSize: number,
): void {
  view.dispatch({
    effects: [indentGuidesCompartment.reconfigure(enabled ? indentGuidesExt(tabSize) : [])],
  });
}
