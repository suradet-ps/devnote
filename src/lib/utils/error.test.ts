import { describe, it, expect } from 'vitest';
import { errorMessage } from './error';

describe('errorMessage', () => {
  it('extracts message from Error', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom');
  });

  it('returns string as-is', () => {
    expect(errorMessage('plain string')).toBe('plain string');
  });

  it('stringifies plain objects', () => {
    expect(errorMessage({ code: 42 })).toBe('{"code":42}');
  });

  it('handles numbers and booleans', () => {
    expect(errorMessage(42)).toBe('42');
    expect(errorMessage(true)).toBe('true');
  });

  it('falls back to String() for unserializable values', () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    // JSON.stringify throws on circular refs; we should fall back
    expect(typeof errorMessage(circular)).toBe('string');
  });
});
