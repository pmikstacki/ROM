// Linux sockaddr_un allows 107 pathname bytes plus its terminating NUL.
// Reserve 48 bytes for Chromium's generated directory and socket filename.
export function requireBrowserSocketBudget(directory) {
  if (typeof directory !== 'string' || !directory.startsWith('/') || directory.includes('\0') || directory.split('/').some(part => part === '..' || part === '.')) throw Error('absolute browser TMPDIR required');
  if (Buffer.byteLength(directory, 'utf8') + 48 > 107) throw Error('browser TMPDIR exceeds Unix socket path budget; use a short owned directory inside the run filesystem');
}
