export function formatPath(path: string, maxLen: number = 40): string {
  if (path.length <= maxLen) return path;
  const separator = path.includes('\\') ? '\\' : '/';
  const parts = path.split(separator);
  if (parts.length <= 3) return path;
  const fileName = parts[parts.length - 1];
  const first = parts[0];
  const lastDir = parts[parts.length - 2];
  return `${first}${separator}...${separator}${lastDir}${separator}${fileName}`;
}

export function getFileName(path: string): string {
  const separator = path.includes('\\') ? '\\' : '/';
  return path.split(separator).pop() ?? 'untitled';
}

export function getDirectory(path: string): string {
  const separator = path.includes('\\') ? '\\' : '/';
  const parts = path.split(separator);
  parts.pop();
  return parts.join(separator) || '/';
}
