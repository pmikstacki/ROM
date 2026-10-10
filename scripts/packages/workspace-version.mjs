// Shared source admission for the workspace's explicit package identity.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

export function requireWorkspaceVersion(root, expected) {
  const source = readFileSync(join(root, 'Cargo.toml'), 'utf8');
  const workspace = source.split(/^\[workspace\.package\]\s*$/m)[1]?.split(/^\[/m)[0];
  const actual = /^version\s*=\s*"([^"]+)"/m.exec(workspace ?? '')?.[1];
  if (typeof expected !== 'string' || !expected || actual !== expected)
    throw Error('workspace package version mismatch');
}
