import { describe, it, expect } from 'vitest';
import { recoveryHash } from './recovery';

describe('recoveryHash', () => {
  it('is stable across saved_at changes (content unchanged → no write)', () => {
    const base = [
      { file_name: 'a.txt', content: 'hello', path: '/a.txt', saved_at: '2026-01-01T00:00:00Z' },
      { file_name: 'untitled-1', content: '', path: null, saved_at: '2026-01-01T00:00:00Z' },
    ];
    const sameContent = [
      { file_name: 'a.txt', content: 'hello', path: '/a.txt', saved_at: '2026-01-02T12:34:56Z' },
      { file_name: 'untitled-1', content: '', path: null, saved_at: '2026-01-02T12:34:56Z' },
    ];
    expect(recoveryHash(sameContent)).toBe(recoveryHash(base));
  });

  it('changes when content changes', () => {
    const before = [{ file_name: 'a.txt', content: 'v1', path: '/a.txt', saved_at: 't' }];
    const after = [{ file_name: 'a.txt', content: 'v2', path: '/a.txt', saved_at: 't' }];
    expect(recoveryHash(after)).not.toBe(recoveryHash(before));
  });

  it('changes when a tab is added or removed', () => {
    const one = [{ file_name: 'a.txt', content: 'x', path: null, saved_at: 't' }];
    const two = [
      { file_name: 'a.txt', content: 'x', path: null, saved_at: 't' },
      { file_name: 'b.txt', content: 'y', path: '/b.txt', saved_at: 't' },
    ];
    expect(recoveryHash(two)).not.toBe(recoveryHash(one));
  });

  it('distinguishes path vs content (same concatenated string)', () => {
    // path="/" content="a" vs path="" content="/a" must hash differently
    const a = [{ file_name: 'x', content: 'a', path: '/', saved_at: 't' }];
    const b = [{ file_name: 'x', content: '/a', path: '', saved_at: 't' }];
    expect(recoveryHash(b)).not.toBe(recoveryHash(a));
  });

  it('empty list has a stable hash', () => {
    expect(recoveryHash([])).toBe(recoveryHash([]));
  });
});
