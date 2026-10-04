// Archive only committed source and the complete compatible skills distribution.
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync, rmSync, mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { assemble } from '../skills/assembly.mjs';
import { capture } from './source.mjs';

export function sourceArchive(root, revision, prefix, output) {
  const tar = execFileSync('git', ['archive', '--format=tar', `--prefix=${prefix}/`, revision], { cwd: root, timeout: 30000, maxBuffer: 128 * 1024 * 1024 });
  const gzip = execFileSync('gzip', ['-n'], { input: tar, timeout: 30000, maxBuffer: 128 * 1024 * 1024 });
  writeFileSync(output, gzip, { flag: 'wx' });
}
export function skillArchive(root, stage, prefix, output) {
  const scratch = mkdtempSync(join(stage, '.skills-'));
  try {
    assemble(root, join(scratch, prefix));
    capture('tar', ['-czf', output, '-C', scratch, prefix], root);
  } finally { rmSync(scratch, { recursive: true, force: true }); }
}
export function directoryArchive(root, prefix, output) {
  const archive = execFileSync('tar', ['-czf', '-', '-C', root, prefix], { timeout: 30000, maxBuffer: 128 * 1024 * 1024 });
  writeFileSync(output, archive, { flag: 'wx' });
}
export function extract(archive, parent, prefix) {
  if (!/^[A-Za-z0-9][A-Za-z0-9._+-]*$/.test(prefix)) throw Error('invalid archive prefix');
  const listing = capture('tar', ['-tzf', archive], parent).trim().split('\n');
  if (!listing.length || listing.some(path => path !== `${prefix}/` && (!path.startsWith(`${prefix}/`) || path.split('/').some(part => part === '..' || part === '.')))) throw Error('invalid archive path');
  const detailed = capture('tar', ['-tvzf', archive], parent).trim().split('\n');
  if (detailed.some(line => !['-', 'd'].includes(line[0]))) throw Error('unsupported archive entry');
  const output = join(parent, prefix);
  if (!listing.includes(`${prefix}/`)) throw Error('missing archive root');
  mkdirSync(output);
  capture('tar', ['-xzf', archive, '-C', parent, '--no-same-owner', '--no-same-permissions'], parent);
  return output;
}
export function archiveCommit(path) {
  const tar = execFileSync('gzip', ['-dc', path], { timeout: 30000, maxBuffer: 128 * 1024 * 1024 });
  // Git consumes two 512-byte records then closes stdin; body writes can produce EPIPE.
  return execFileSync('git', ['get-tar-commit-id'], { input: tar.subarray(0, 1024), timeout: 10000, encoding: 'utf8' }).trim();
}
