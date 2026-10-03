// Git itself reconstructs an extracted tree without filters or checkout configuration.
import { execFileSync } from 'node:child_process';
import { lstatSync } from 'node:fs';
import { join } from 'node:path';

export function extractedTree(root, paths, scratch, format) {
  if (!['sha1', 'sha256'].includes(format)) throw Error('unsupported Git object format');
  const repository = join(scratch, 'verification.git');
  const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith('GIT_')));
  env.GIT_CONFIG_NOSYSTEM = '1'; env.GIT_CONFIG_GLOBAL = '/dev/null';
  const run = (args, input) => execFileSync('git', args, { cwd: scratch, input, env, timeout: 10000, maxBuffer: 32 * 1024 * 1024, encoding: 'utf8' });
  run(['init', '--bare', '--quiet', `--object-format=${format}`, repository]);
  const base = ['--git-dir', repository];
  const hashes = run([...base, 'hash-object', '-w', '--no-filters', '--stdin-paths'], paths.map(path => JSON.stringify(join(root, path))).join('\n') + '\n').trim().split('\n');
  const entries = paths.map((path, index) => `${lstatSync(join(root, path)).mode & 0o111 ? '100755' : '100644'} ${hashes[index]}\t${path}`).join('\n') + '\n';
  run([...base, 'update-index', '--index-info'], entries);
  return run([...base, 'write-tree']).trim();
}
