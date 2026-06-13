import { javascript } from '@codemirror/lang-javascript';
import { LanguageSupport, StreamLanguage } from '@codemirror/language';
import type { StreamParser } from '@codemirror/language';

/**
 * Cache for dynamically-loaded language supports.
 * Keyed by `${lang}` so distinct variants don't collide.
 */
const cache = new Map<string, LanguageSupport | Promise<LanguageSupport>>();

function getCached(
  key: string,
  loader: () => Promise<LanguageSupport>,
): Promise<LanguageSupport> {
  const existing = cache.get(key);
  if (existing) return Promise.resolve(existing);
  const p = loader();
  cache.set(key, p);
  // Once resolved, swap the promise for the value so future calls are sync
  p.then((v) => cache.set(key, v)).catch(() => cache.delete(key));
  return p;
}

function stream(s: StreamParser<unknown>): LanguageSupport {
  return new LanguageSupport(StreamLanguage.define(s));
}

/**
 * Returns a CodeMirror LanguageSupport for the given key.
 *
 * - For JavaScript: returns synchronously (the only eagerly-loaded pack).
 * - For all other languages: returns a Promise that resolves on first
 *   request; subsequent requests are served from a cache.
 * - On unknown keys: falls back to JavaScript synchronously.
 *
 * CodeMirror's EditorState.create is synchronous, so the caller should
 * either:
 *  1. Use {@link reconfigureLanguage} in setup.ts (which handles promises), or
 *  2. Pre-warm the cache via {@link preloadLanguage} before constructing state.
 */
export function getLanguage(lang: string): LanguageSupport | Promise<LanguageSupport> {
  switch (lang) {
    case 'javascript':
      return javascript();
    case 'typescript':
      return getCached('typescript', () =>
        import('@codemirror/lang-javascript').then((m) => m.javascript({ typescript: true })),
      );
    case 'rust':
      return getCached('rust', () => import('@codemirror/lang-rust').then((m) => m.rust()));
    case 'python':
      return getCached('python', () => import('@codemirror/lang-python').then((m) => m.python()));
    case 'html':
      return getCached('html', () => import('@codemirror/lang-html').then((m) => m.html()));
    case 'css':
      return getCached('css', () => import('@codemirror/lang-css').then((m) => m.css()));
    case 'json':
      return getCached('json', () => import('@codemirror/lang-json').then((m) => m.json()));
    case 'markdown':
      return getCached('markdown', () => import('@codemirror/lang-markdown').then((m) => m.markdown()));
    case 'sql':
      return getCached('sql', () => import('@codemirror/lang-sql').then((m) => m.sql()));
    case 'xml':
      return getCached('xml', () => import('@codemirror/lang-xml').then((m) => m.xml()));
    case 'cpp':
      return getCached('cpp', () => import('@codemirror/lang-cpp').then((m) => m.cpp()));
    case 'java':
      return getCached('java', () => import('@codemirror/lang-java').then((m) => m.java()));
    case 'php':
      return getCached('php', () => import('@codemirror/lang-php').then((m) => m.php()));
    case 'bash':
      return getCached('bash', () =>
        import('@codemirror/legacy-modes/mode/shell').then(({ shell }) => stream(shell)),
      );
    case 'go':
      return getCached('go', () =>
        import('@codemirror/legacy-modes/mode/go').then(({ go }) => stream(go)),
      );
    case 'ruby':
      return getCached('ruby', () =>
        import('@codemirror/legacy-modes/mode/ruby').then(({ ruby }) => stream(ruby)),
      );
    case 'yaml':
      return getCached('yaml', () =>
        import('@codemirror/legacy-modes/mode/yaml').then(({ yaml }) => stream(yaml)),
      );
    case 'toml':
      return getCached('toml', () =>
        import('@codemirror/legacy-modes/mode/toml').then(({ toml }) => stream(toml)),
      );
    case 'vue':
      return getCached('html', () => import('@codemirror/lang-html').then((m) => m.html()));
    default:
      return javascript();
  }
}

/**
 * Pre-warm the cache for a given language. Returns when the pack is loaded
 * (or immediately if already cached). Use this from `onMount` of the editor
 * if you need the language to be available synchronously.
 */
export function preloadLanguage(lang: string): Promise<LanguageSupport | null> {
  const v = getLanguage(lang);
  return Promise.resolve(v).then((r) => (r instanceof Promise ? r : r));
}
