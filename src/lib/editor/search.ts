/**
 * Pure, DOM-free search/replace semantics for the Find & Replace panel.
 *
 * The editor wires these into CodeMirror transactions (see Editor.svelte);
 * keeping them here means the match-count, navigation and replace-all
 * semantics can be asserted without an editor instance.
 */

export interface SearchOptions {
  caseSensitive?: boolean;
  useRegex?: boolean;
}

export interface SearchMatch {
  from: number;
  to: number;
}

function buildRegex(query: string, opts: SearchOptions): RegExp | null {
  if (!query) return null;
  const source = opts.useRegex ? query : escapeRegExp(query);
  const flags = opts.caseSensitive ? 'g' : 'gi';
  try {
    return new RegExp(source, flags);
  } catch {
    return null; // invalid regex → no matches, never throw
  }
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** All non-overlapping matches of `query` in `content`, in order. */
export function findAll(content: string, query: string, opts: SearchOptions = {}): SearchMatch[] {
  const re = buildRegex(query, opts);
  if (!re) return [];
  const matches: SearchMatch[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(content)) !== null) {
    if (m[0].length === 0) {
      // Guard against zero-length matches (e.g. `/x*/`) looping forever
      re.lastIndex++;
      continue;
    }
    matches.push({ from: m.index, to: m.index + m[0].length });
  }
  return matches;
}

export function countMatches(content: string, query: string, opts: SearchOptions = {}): number {
  return findAll(content, query, opts).length;
}

/** First match at or after `from`; null when there is none. */
export function findNextFrom(
  content: string,
  query: string,
  from: number,
  opts: SearchOptions = {},
): SearchMatch | null {
  return findAll(content, query, opts).find((m) => m.from >= from) ?? null;
}

export interface ReplaceAllResult {
  content: string;
  count: number;
}

/**
 * Replace every non-overlapping occurrence of `query` in `content`.
 * Literal replacements are `$`-safe; regex replacements support `$1`, `$&`, …
 * Invalid regexes and empty queries return the input unchanged.
 */
export function replaceAll(
  content: string,
  query: string,
  replacement: string,
  opts: SearchOptions = {},
): ReplaceAllResult {
  const re = buildRegex(query, opts);
  if (!re) return { content, count: 0 };

  const matches = content.match(re);
  const count = matches ? matches.filter((m) => m.length > 0).length : 0;
  if (count === 0) return { content, count: 0 };

  const escaped = opts.useRegex ? replacement : replacement.replace(/\$/g, '$$$$');
  return { content: content.replace(re, escaped), count };
}
