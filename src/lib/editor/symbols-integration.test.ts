// @vitest-environment happy-dom
import { describe, it, expect } from 'vitest';
import { EditorView } from '@codemirror/view';
import { createEditorState, reconfigureLanguage } from '../codemirror/setup';
import { extractSymbols } from './symbols';
import type { Settings } from '$lib/stores/settings.svelte';

const settings: Settings = {
  theme: 'light',
  fontSize: 14,
  fontFamily: 'JetBrains Mono',
  wordWrap: false,
  showLineNumbers: true,
  showStatusBar: true,
  tabSize: 4,
  insertSpaces: true,
  locale: 'en',
};

/** Wait for the async language pack + reconfigure dispatch to settle. */
async function settle(): Promise<void> {
  await new Promise((r) => setTimeout(r, 0));
  await new Promise((r) => setTimeout(r, 0));
}

/**
 * Integration smoke: the real app flow — createEditorState with an async
 * language pack, reconfigureLanguage like the editor does on mount, then
 * extract symbols. Guards the go-to-symbol runtime path.
 */
describe('go-to-symbol runtime flow', () => {
  it('extracts symbols after an async language pack resolves', async () => {
    const state = createEditorState(
      'fn main() {}\nstruct Foo {}\nfn helper() {}\n',
      settings,
      'light',
      'rust',
      () => {},
      () => {},
      () => {},
    );
    const view = new EditorView({ state, parent: document.body });
    reconfigureLanguage(view, 'rust'); // fire-and-forget, like the app
    await settle();
    const symbols = extractSymbols(view.state);
    const names = symbols.map((s) => s.name);
    expect(names).toContain('main');
    expect(names).toContain('Foo');
    expect(names).toContain('helper');
    view.destroy();
  });

  it('extracts symbols for typescript after async load', async () => {
    const state = createEditorState(
      'interface Shape {}\ntype Id = string;\nclass Box implements Shape {}\n',
      settings,
      'light',
      'typescript',
      () => {},
      () => {},
      () => {},
    );
    const view = new EditorView({ state, parent: document.body });
    reconfigureLanguage(view, 'typescript');
    await settle();
    const names = extractSymbols(view.state).map((s) => s.name);
    expect(names).toContain('Shape');
    expect(names).toContain('Id');
    expect(names).toContain('Box');
    view.destroy();
  });

  it('finds symbols past the 3000-char synchronous parse window', async () => {
    const filler = '// padding line\n'.repeat(250); // ~3750 chars of comments
    const doc = `${filler}fn late_symbol() {}\n`;
    const state = createEditorState(doc, settings, 'light', 'rust', () => {}, () => {}, () => {});
    const view = new EditorView({ state, parent: document.body });
    reconfigureLanguage(view, 'rust');
    await settle();
    const names = extractSymbols(view.state).map((s) => s.name);
    expect(names).toContain('late_symbol');
    view.destroy();
  });

  it('returns [] for plain text (no parser)', async () => {
    const state = createEditorState(
      'just some text\nnothing to find\n',
      settings,
      'light',
      'text',
      () => {},
      () => {},
      () => {},
    );
    const view = new EditorView({ state, parent: document.body });
    reconfigureLanguage(view, 'text');
    await settle();
    expect(extractSymbols(view.state)).toEqual([]);
    view.destroy();
  });
});
