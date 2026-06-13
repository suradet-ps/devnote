import { describe, it, expect } from 'vitest';
import { ensureExtension } from './ensure-ext';

describe('ensureExtension', () => {
  it('appends default extension when filename has no dot', () => {
    expect(ensureExtension('/a/b/my_note')).toBe('/a/b/my_note.txt');
    expect(ensureExtension('/Users/me/notes', 'md')).toBe('/Users/me/notes.md');
  });

  it('keeps hidden files (dotfiles) as-is', () => {
    expect(ensureExtension('/a/.env')).toBe('/a/.env');
    expect(ensureExtension('/Users/me/.gitignore')).toBe('/Users/me/.gitignore');
  });

  it('keeps filenames that already have an extension', () => {
    expect(ensureExtension('/a/notes.md')).toBe('/a/notes.md');
    expect(ensureExtension('/a/archive.tar.gz')).toBe('/a/archive.tar.gz');
  });

  it('handles Windows-style paths', () => {
    expect(ensureExtension('C:\\Users\\me\\notes')).toBe('C:\\Users\\me\\notes.txt');
    expect(ensureExtension('C:\\Users\\me\\notes.md')).toBe('C:\\Users\\me\\notes.md');
  });

  it('handles empty path gracefully', () => {
    expect(ensureExtension('')).toBe('');
  });
});
