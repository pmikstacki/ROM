// Bind fixture source and retained output bytes; do not execute generated programs.
import { lstatSync, readdirSync, realpathSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { safePath, hash } from '../skills/files.mjs';

const inputs = ['src', 'tests', 'package.json', 'package-lock.json', 'tsconfig.json', 'vite.config.ts', 'index.html'];
function inventory(root, selected) {
  if (typeof root !== 'string' || !isAbsolute(root) || resolve(root) !== root || realpathSync(root) !== root ||
      !lstatSync(root).isDirectory()) throw Error('invalid fixture directory');
  const files = {};
  let visited = 0, bytes = 0;
  function visit(path, depth = 0) {
    if (++visited > 100_000 || depth > 32) throw Error('fixture inventory exceeds bound');
    const full = safePath(root, path), stat = lstatSync(full);
    if (stat.isDirectory()) {
      for (const entry of readdirSync(full).sort()) visit(`${path}/${entry}`, depth + 1);
    } else {
      if (!stat.isFile() || stat.nlink !== 1 || stat.size > 64 * 1024 * 1024) throw Error('invalid fixture file');
      bytes += stat.size;
      if (bytes > 256 * 1024 * 1024) throw Error('fixture byte count exceeds bound');
      files[path] = hash(full);
    }
  }
  for (const path of selected) visit(path);
  return files;
}

/** lease is the verified extraction still held by the producer or static admission. */
export function requireRecoveryFixture(lease, directory, record) {
  try {
    if (!lease || typeof lease.fence !== 'function' || !record) throw Error('invalid fixture source');
    lease.fence();
    const native = inventory(lease.root, ['tests/recovery-host']);
    if (!isDeepStrictEqual(record.fixture_source_sha256, native)) throw Error('native fixture source differs');
    const verifier = inventory(lease.root, ['studio/tests/mutation-recovery/verify.mjs']);
    if (record.verifier_sha256 !== verifier['studio/tests/mutation-recovery/verify.mjs']) throw Error('fixture verifier differs');
    const output = inventory(directory, [...inputs.map(path => `consumer/${path}`), 'consumer/dist', 'consumer/http-dist', 'runtime/proxy.mjs', 'runtime/server.mjs']);
    if (!isDeepStrictEqual(record.consumer_fixture_sha256, output)) throw Error('retained fixture output differs');
    const source = inventory(lease.root, inputs.map(path => `studio/tests/mutation-recovery/consumer/${path}`));
    const runtime = inventory(lease.root, ['studio/tests/mutation-recovery/proxy.mjs', 'studio/tests/mutation-recovery/server.mjs']);
    for (const [path, digest] of Object.entries(source)) {
      if (output[path.slice('studio/tests/mutation-recovery/'.length)] !== digest) throw Error('consumer fixture input differs');
    }
    for (const [path, digest] of Object.entries(runtime)) {
      if (output[`runtime/${path.split('/').at(-1)}`] !== digest) throw Error('consumer runtime source differs');
    }
    lease.fence();
    return record;
  } catch (error) {
    throw Error('invalid recovery fixture evidence', { cause: error });
  }
}
