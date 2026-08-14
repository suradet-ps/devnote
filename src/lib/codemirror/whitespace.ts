import { Compartment, RangeSetBuilder, type Extension } from '@codemirror/state';
import {
  Decoration,
  EditorView,
  ViewPlugin,
  type DecorationSet,
  type ViewUpdate,
} from '@codemirror/view';

/**
 * Visible whitespace (Roadmap Phase 4): renders spaces as `·` and tabs as `→`
 * within the visible viewport only. Pure decoration — the document is
 * untouched; toggled via a Compartment.
 */

const spaceMark = Decoration.mark({ class: 'cm-ws-space' });
const tabMark = Decoration.mark({ class: 'cm-ws-tab' });

function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos < to) {
      const line = view.state.doc.lineAt(pos);
      const end = Math.min(line.to, to);
      const text = view.state.sliceDoc(line.from, end);
      for (let i = 0; i < text.length; i++) {
        const ch = text[i];
        if (ch === ' ') {
          builder.add(line.from + i, line.from + i + 1, spaceMark);
        } else if (ch === '\t') {
          builder.add(line.from + i, line.from + i + 1, tabMark);
        }
      }
      pos = end + 1;
    }
  }
  return builder.finish();
}

export const visibleWhitespaceExt: Extension = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }
    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  { decorations: (v) => v.decorations },
);

export const whitespaceCompartment = new Compartment();

export function reconfigureVisibleWhitespace(view: EditorView, enabled: boolean): void {
  view.dispatch({
    effects: [whitespaceCompartment.reconfigure(enabled ? visibleWhitespaceExt : [])],
  });
}
