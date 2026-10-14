export function normalizePath(path: string, windows: boolean): string {
  if (!windows) return String(path || '');
  return String(path || '').replace(/^\\\\\?\\/, '').replaceAll('\\', '/').toLowerCase();
}
export function commandText(args: string[], windows: boolean): string {
  if(windows) return `& ${args.map(arg=>`'${arg.replaceAll("'", "''")}'`).join(' ')}`;
  return args.map(arg=>`'${arg.replaceAll("'", "'\\''")}'`).join(' ');
}
export function resolveModelPath(model: string, directory: string, windows: boolean): string {
  const absolute = windows ? /^(?:[A-Za-z]:|[\\/])/.test(model) : model.startsWith('/');
  const trim = windows ? /[\\/]+$/ : /\/+$/;
  return normalizePath(absolute || !directory ? model : `${directory.replace(trim, '')}/${model}`,windows);
}
export function parentPath(path: string, windows: boolean): string {
  return path.replace(windows ? /[\\/][^\\/]+$/ : /\/[^/]+$/, '');
}
export function baseName(path: string, windows: boolean): string {
  return path.split(windows ? /[\\/]/ : /\//).at(-1) || '';
}
