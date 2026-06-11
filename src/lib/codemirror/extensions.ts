import { javascript } from '@codemirror/lang-javascript';
import { rust } from '@codemirror/lang-rust';
import { python } from '@codemirror/lang-python';
import { html } from '@codemirror/lang-html';
import { css } from '@codemirror/lang-css';
import { json } from '@codemirror/lang-json';
import { markdown } from '@codemirror/lang-markdown';
import { sql } from '@codemirror/lang-sql';
import { xml } from '@codemirror/lang-xml';
import type { LanguageSupport } from '@codemirror/language';

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
};

export function getLanguage(lang: string): LanguageSupport {
  const factory = languageMap[lang];
  return factory ? factory() : javascript();
}
