import { describe, it, expect } from 'vitest';
import { findAll, countMatches, findNextFrom, replaceAll } from './search';

describe('findAll', () => {
  it('finds literal matches case-insensitively by default', () => {
    const m = findAll('Hello hello HELLO', 'hello');
    expect(m).toEqual([
      { from: 0, to: 5 },
      { from: 6, to: 11 },
      { from: 12, to: 17 },
    ]);
  });

  it('respects caseSensitive', () => {
    expect(findAll('Hello hello', 'hello', { caseSensitive: true })).toEqual([
      { from: 6, to: 11 },
    ]);
  });

  it('supports regex', () => {
    const m = findAll('a1 b2 c3', '\\d', { useRegex: true });
    expect(m.map((x) => x.to - x.from)).toEqual([1, 1, 1]);
    expect(m[0]).toEqual({ from: 1, to: 2 });
  });

  it('returns [] for empty query, invalid regex, or no match', () => {
    expect(findAll('abc', '')).toEqual([]);
    expect(findAll('abc', '(')).toEqual([]);
    expect(findAll('abc', 'xyz')).toEqual([]);
  });

  it('skips zero-length matches without hanging', () => {
    const m = findAll('abc', 'x*');
    expect(m).toEqual([]);
  });
});

describe('countMatches', () => {
  it('counts occurrences', () => {
    expect(countMatches('a, b, a, c', 'a')).toBe(2);
    expect(countMatches('a, b, a, c', 'A')).toBe(2);
    expect(countMatches('a, b, a, c', 'A', { caseSensitive: true })).toBe(0);
    expect(countMatches('a, b', '')).toBe(0);
  });
});

describe('findNextFrom', () => {
  it('returns the first match at or after the position', () => {
    const m = findNextFrom('a1 b2 c3', '\\d', 4, { useRegex: true });
    expect(m).toEqual({ from: 4, to: 5 });
  });

  it('returns null when no match remains', () => {
    expect(findNextFrom('a1 b2', '\\d', 6, { useRegex: true })).toBeNull();
    expect(findNextFrom('abc', 'x', 0)).toBeNull();
  });
});

describe('replaceAll', () => {
  it('replaces all literal matches and reports the count', () => {
    const r = replaceAll('one two one three one', 'one', '1');
    expect(r.content).toBe('1 two 1 three 1');
    expect(r.count).toBe(3);
  });

  it('keeps literal replacements $-safe', () => {
    const r = replaceAll('cost: $5', '$5', 'X');
    expect(r.content).toBe('cost: X');
  });

  it('supports regex capture groups', () => {
    const r = replaceAll('name=Alice; name=Bob', 'name=(\\w+)', 'user:$1', {
      useRegex: true,
    });
    expect(r.content).toBe('user:Alice; user:Bob');
  });

  it('supports $& for the full match', () => {
    const r = replaceAll('a-b', '-', '<$&>', { useRegex: true });
    expect(r.content).toBe('a<->b');
  });

  it('respects caseSensitive', () => {
    const r = replaceAll('A a A', 'a', 'X', { caseSensitive: true });
    expect(r.content).toBe('A X A');
    expect(r.count).toBe(1);
  });

  it('returns the input unchanged for empty query, invalid regex or no match', () => {
    expect(replaceAll('abc', '', 'x')).toEqual({ content: 'abc', count: 0 });
    expect(replaceAll('abc', '(', 'x')).toEqual({ content: 'abc', count: 0 });
    expect(replaceAll('abc', 'z', 'x')).toEqual({ content: 'abc', count: 0 });
  });
});
