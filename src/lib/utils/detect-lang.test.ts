import { describe, it, expect } from 'vitest';
import { detectLanguage } from './detect-lang';

describe('detectLanguage', () => {
  it.each([
    ['main.rs', 'rust'],
    ['app.tsx', 'typescript'],
    ['script.py', 'python'],
    ['README.md', 'markdown'],
    ['package.json', 'json'],
    ['Cargo.toml', 'toml'],
    ['go.mod', 'text'],
    ['config.yaml', 'yaml'],
    ['config.yml', 'yaml'],
    ['index.html', 'html'],
    ['styles.css', 'css'],
    ['data.sql', 'sql'],
    ['doc.xml', 'xml'],
    ['app.vue', 'vue'],
    ['main.cpp', 'cpp'],
    ['Main.java', 'java'],
    ['app.php', 'php'],
    ['script.sh', 'bash'],
    ['script.bash', 'bash'],
    ['build.zsh', 'bash'],
  ])('maps %s → %s', (path, expected) => {
    expect(detectLanguage(path)).toBe(expected);
  });

  it('returns "text" for null path with no content', () => {
    expect(detectLanguage(null)).toBe('text');
  });

  it('falls back to shebang for untitled files', () => {
    expect(detectLanguage(null, '#!/usr/bin/env python3\nprint(1)')).toBe('python');
    expect(detectLanguage(null, '#!/usr/bin/env node\nconsole.log(1)')).toBe('javascript');
    expect(detectLanguage(null, '#!/bin/bash\necho hi')).toBe('bash');
    expect(detectLanguage(null, '#!/usr/bin/env ruby\nputs 1')).toBe('ruby');
  });

  it('returns "text" for shebang-less content', () => {
    expect(detectLanguage(null, 'hello world')).toBe('text');
  });

  it('handles uppercase extensions', () => {
    expect(detectLanguage('Main.RS')).toBe('rust');
    expect(detectLanguage('Script.PY')).toBe('python');
  });

  it('returns "text" for unknown extensions', () => {
    expect(detectLanguage('mystery.xyz')).toBe('text');
    expect(detectLanguage('noext')).toBe('text');
  });
});
