// Frozen pnpm locks determine dependency placement; this check verifies realized package versions.
import { existsSync, readFileSync, readdirSync, realpathSync } from 'node:fs';
import { join, relative } from 'node:path';

export function assertPnpmVersions(fixture, seedLock) {
  const modules = realpathSync(join(fixture, 'node_modules'));
  const expected = new Map();
  for (const [path, entry] of Object.entries(seedLock.packages)) {
    if (!path || !entry.version) continue;
    const name = entry.name ?? path.split('node_modules/').at(-1);
    const key = name + '@' + entry.version;
    expected.set(key, { optional: expected.has(key) ? expected.get(key).optional && !!entry.optional : !!entry.optional });
  }
  const found = new Set(), visited = new Set();
  function packageAt(path) {
    const actual = realpathSync(path), rel = relative(modules, actual);
    if (rel === '..' || rel.startsWith('../') || rel.startsWith('/')) throw Error('dependency is linked outside the consumer installation');
    if (visited.has(actual)) return;
    if (visited.size >= 10000) throw Error('installed package inventory exceeds bound');
    visited.add(actual);
    const pkg = JSON.parse(readFileSync(join(actual, 'package.json'), 'utf8'));
    const key = pkg.name + '@' + pkg.version;
    if (!expected.has(key)) throw Error('unexpected installed package version: ' + key);
    found.add(key);
    const nested = join(actual, 'node_modules');
    if (existsSync(nested)) scan(nested);
  }
  function scan(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.name.startsWith('.') || (!entry.isDirectory() && !entry.isSymbolicLink())) continue;
      const path = join(directory, entry.name);
      if (entry.name.startsWith('@')) {
        for (const scoped of readdirSync(path, { withFileTypes: true }))
          if (scoped.isDirectory() || scoped.isSymbolicLink()) packageAt(join(path, scoped.name));
      } else packageAt(path);
    }
  }
  scan(modules);
  for (const [key, entry] of expected) if (!entry.optional && !found.has(key)) throw Error('missing installed package version: ' + key);
  return [...found].sort();
}
