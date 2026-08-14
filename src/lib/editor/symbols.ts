import type { EditorState } from '@codemirror/state';
import { ensureSyntaxTree, syntaxTree } from '@codemirror/language';
import type { SyntaxNode } from '@lezer/common';

/**
 * Go-to-Symbol (Roadmap Phase 4): extracts definition sites from CodeMirror's
 * parsed syntax tree. Editor-local only — no workspace indexing. Falls back
 * to an empty list for languages without a real parser.
 *
 * Node names are the lezer grammars' (PascalCase): lezer-rust, lezer-javascript,
 * lezer-python.
 */

export interface SymbolInfo {
  name: string;
  line: number;
  pos: number;
  type: string;
}

/** Node types that look like definitions, per parsed language family. */
function isDefinitionNode(type: string): boolean {
  // Rust (lezer-rust)
  if (
    /^(Function|Struct|Enum|Trait|Impl|Module|Const|Static|Type)Item$/.test(type) ||
    type === 'MacroRules'
  ) {
    return true;
  }
  // JavaScript / TypeScript (lezer-javascript / lezer-typescript)
  if (
    type === 'FunctionDeclaration' ||
    type === 'ClassDeclaration' ||
    type === 'MethodDeclaration' ||
    type === 'InterfaceDeclaration' ||
    type === 'TypeAliasDeclaration' ||
    type === 'EnumDeclaration'
  ) {
    return true;
  }
  // Python (lezer-python)
  if (
    type === 'FunctionDefinition' ||
    type === 'AsyncFunctionDefinition' ||
    type === 'ClassDefinition'
  ) {
    return true;
  }
  return false;
}

/** Name-bearing child node types, in priority order. */
const NAME_CHILD_TYPES = [
  'VariableName', // python def/class names
  'VariableDefinition', // js/ts function + class names
  'TypeDefinition', // ts interfaces / type aliases
  'TypeName',
  'PropertyDefinition', // js/ts method names
  'PropertyName',
  'BoundIdentifier', // rust fn/const names
  'TypeIdentifier', // rust struct/enum/trait/impl names
  'Identifier',
  'identifier',
] as const;

/** Best-effort name: the declaration's name child, else the first line. */
function nodeName(state: EditorState, node: SyntaxNode): string {
  for (const childType of NAME_CHILD_TYPES) {
    const child = node.getChildren(childType)[0];
    if (child) {
      return state.sliceDoc(child.from, child.to);
    }
  }
  const firstLine = state
    .sliceDoc(node.from, node.to)
    .split(/[\n{]/)[0]
    .trim();
  return firstLine.slice(0, 60) || '(unnamed)';
}

export function extractSymbols(state: EditorState): SymbolInfo[] {
  // lezer only parses the first ~3000 chars synchronously when a language is
  // (re)configured; the rest is parsed lazily by the view. ensureSyntaxTree
  // forces the parse forward (budgeted) and returns the complete tree — the
  // state field itself keeps the stale partial tree.
  const tree = ensureSyntaxTree(state, state.doc.length, 250) ?? syntaxTree(state);

  const symbols: SymbolInfo[] = [];
  const cursor = tree.cursor();

  do {
    const node = cursor.node;
    if (!isDefinitionNode(node.type.name)) continue;
    symbols.push({
      name: nodeName(state, node),
      line: state.doc.lineAt(node.from).number,
      pos: node.from,
      type: node.type.name,
    });
  } while (cursor.next());

  return symbols;
}
