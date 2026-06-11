export function detectLanguage(path: string | null): string {
  if (!path) return 'text';
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  const extMap: Record<string, string> = {
    rs: 'rust', ts: 'typescript', tsx: 'typescript',
    js: 'javascript', jsx: 'javascript',
    py: 'python', html: 'html', htm: 'html',
    css: 'css', scss: 'css', less: 'css',
    md: 'markdown', json: 'json', sql: 'sql',
    xml: 'xml', vue: 'vue', svelte: 'javascript',
    cpp: 'cpp', cc: 'cpp', c: 'cpp', h: 'cpp',
    java: 'java', php: 'php',
    toml: 'toml', yaml: 'yaml', yml: 'yaml',
    txt: 'text',
  };
  return extMap[ext] ?? 'text';
}
