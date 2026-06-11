export function formatPath(path: string, maxLen: number = 40): string {
  if (path.length <= maxLen) return path;
  const parts = path.split(/[/\\]/);
  if (parts.length <= 2) return path;
  const fileName = parts[parts.length - 1];
  const dir = parts.slice(0, 2).join('/');
  return `${dir}/.../${fileName}`;
}
