import { javascript } from '@codemirror/lang-javascript';
import { rust } from '@codemirror/lang-rust';
import { python } from '@codemirror/lang-python';
import { html } from '@codemirror/lang-html';
import { css } from '@codemirror/lang-css';
import { json } from '@codemirror/lang-json';
import { markdown } from '@codemirror/lang-markdown';
import { sql } from '@codemirror/lang-sql';
import { xml } from '@codemirror/lang-xml';
import { cpp } from '@codemirror/lang-cpp';
import { java } from '@codemirror/lang-java';
import { php } from '@codemirror/lang-php';
import { StreamLanguage, LanguageSupport } from '@codemirror/language';
import { yaml as yamlMode } from '@codemirror/legacy-modes/mode/yaml';
import { toml as tomlMode } from '@codemirror/legacy-modes/mode/toml';
import { shell as shellMode } from '@codemirror/legacy-modes/mode/shell';
import { go as goMode } from '@codemirror/legacy-modes/mode/go';
import { ruby as rubyMode } from '@codemirror/legacy-modes/mode/ruby';

const languageMap: Record<string, () => LanguageSupport> = {
  javascript: () => javascript(),
  typescript: () => javascript({ typescript: true }),
  rust: () => rust(),
  python: () => python(),
  html: () => html(),
  css: () => css(),
  json: () => json(),
  markdown: () => markdown(),
  sql: () => sql(),
  xml: () => xml(),
  vue: () => html(),
  cpp: () => cpp(),
  java: () => java(),
  php: () => php(),
  bash: () => new LanguageSupport(StreamLanguage.define(shellMode)),
  go: () => new LanguageSupport(StreamLanguage.define(goMode)),
  ruby: () => new LanguageSupport(StreamLanguage.define(rubyMode)),
  yaml: () => new LanguageSupport(StreamLanguage.define(yamlMode)),
  toml: () => new LanguageSupport(StreamLanguage.define(tomlMode)),
};

export function getLanguage(lang: string): LanguageSupport {
  const factory = languageMap[lang];
  return factory ? factory() : javascript();
}
