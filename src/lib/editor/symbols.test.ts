import { describe, it, expect } from 'vitest';
import { EditorState } from '@codemirror/state';
import { rust } from '@codemirror/lang-rust';
import { javascript } from '@codemirror/lang-javascript';
import { python } from '@codemirror/lang-python';
import { extractSymbols } from './symbols';

function stateFor(doc: string, lang: unknown): EditorState {
  return EditorState.create({ doc, extensions: [lang as never] });
}

describe('extractSymbols', () => {
  it('finds Rust items with their names', () => {
    const state = stateFor(
      [
        'fn main() {}',
        'struct Foo { x: i32 }',
        'enum Bar { A, B }',
        'trait Baz {}',
        'impl Foo { fn method(&self) {} }',
        'const LIMIT: i32 = 10;',
      ].join('\n'),
      rust(),
    );
    const names = extractSymbols(state).map((s) => s.name);
    expect(names).toContain('main');
    expect(names).toContain('Foo');
    expect(names).toContain('Bar');
    expect(names).toContain('Baz');
    expect(names).toContain('method');
    expect(names).toContain('LIMIT');
  });

  it('reports line numbers', () => {
    const state = stateFor('fn a() {}\n\nfn b() {}\n', rust());
    const symbols = extractSymbols(state);
    expect(symbols.find((s) => s.name === 'a')?.line).toBe(1);
    expect(symbols.find((s) => s.name === 'b')?.line).toBe(3);
  });

  it('finds JavaScript functions and classes', () => {
    const state = stateFor(
      ['function helper() {}', 'class Widget {', '  constructor() {}', '  render() {}', '}', 'const x = 1;'].join('\n'),
      javascript(),
    );
    const names = extractSymbols(state).map((s) => s.name);
    expect(names).toContain('helper');
    expect(names).toContain('Widget');
    expect(names).toContain('constructor');
    expect(names).toContain('render');
  });

  it('finds Python definitions', () => {
    const state = stateFor(
      ['def greet(name):', '    return name', 'class Person:', '    def __init__(self):', '        pass'].join('\n'),
      python(),
    );
    const names = extractSymbols(state).map((s) => s.name);
    expect(names).toContain('greet');
    expect(names).toContain('Person');
    expect(names).toContain('__init__');
  });

  it('returns [] for empty docs and non-parsed languages', () => {
    expect(extractSymbols(stateFor('', rust()))).toEqual([]);
  });
});
