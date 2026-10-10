import { openSync, closeSync, fstatSync, readSync, lstatSync, realpathSync, constants } from 'node:fs';
export const nativeTlsVolume = '/var/tmp/rom-010-authentik-20261007/run/volume';
export function nativeTlsPrivatePath(path) {
  if (typeof path !== 'string' || !path.startsWith(`${nativeTlsVolume}/private/native-tls-`) || path.split('/').includes('..')) throw Error('owned native TLS private path required');
  return path;
}
export function readNativeTlsFile(path, maximum = 65536, privateFile = false) {
  if (realpathSync(path) !== path) throw Error('canonical native TLS input required');
  const before = lstatSync(path);
  if (!before.isFile() || before.size > maximum || (privateFile && (before.nlink !== 1 || (before.mode & 0o077) !== 0))) throw Error('bounded private regular input required');
  const descriptor = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const opened = fstatSync(descriptor);
    if (opened.dev !== before.dev || opened.ino !== before.ino || opened.size !== before.size) throw Error('input replaced');
    const bytes = Buffer.alloc(before.size + 1); let length = 0;
    while (length < bytes.length) { const count = readSync(descriptor, bytes, length, bytes.length - length, null); if (!count) break; length += count; }
    const after = fstatSync(descriptor);
    if (length !== before.size || after.size !== opened.size || after.mtimeMs !== opened.mtimeMs || after.ctimeMs !== opened.ctimeMs) throw Error('input changed');
    return bytes.subarray(0, length);
  } finally { closeSync(descriptor); }
}
export function readNativeTlsJson(path, maximum = 65536, privateFile = true) { return JSON.parse(readNativeTlsFile(path, maximum, privateFile)); }

// Read only the explicitly owned target's finite proc environment.
export function readNativeTlsEnvironment(pid) {
  if (!Number.isSafeInteger(pid) || pid < 2) throw Error('owned target pid required');
  const descriptor = openSync(`/proc/${pid}/environ`, 'r');
  try {
    const bytes = Buffer.alloc(65537); let length = 0;
    while (length < bytes.length) { const count = readSync(descriptor, bytes, length, bytes.length - length, null); if (!count) break; length += count; }
    if (length > 65536) throw Error('child environment exceeded bound');
    const values = bytes.subarray(0, length).toString().split('\0').filter(Boolean);
    const entries = values.map(value => { const offset = value.indexOf('='); if (offset < 1) throw Error('malformed child environment'); return [value.slice(0, offset), value.slice(offset + 1)]; });
    if (new Set(entries.map(([name]) => name)).size !== entries.length) throw Error('duplicate child environment variable');
    return Object.fromEntries(entries);
  } finally { closeSync(descriptor); }
}
