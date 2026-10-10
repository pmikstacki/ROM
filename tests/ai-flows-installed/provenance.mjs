// Read-only identity checks for externally selected producer inputs and copied consumers.
import { readFileSync, lstatSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { hash, digest, safePath } from '../../scripts/skills/files.mjs';
import { contents } from '../../scripts/release-artifacts/contents.mjs';
import { inventory } from '../../scripts/packages/astral-adoption.mjs';
import { requireWorkspaceVersion } from '../../scripts/packages/workspace-version.mjs';

export function fenceInputs(root, expected) {
  if (!isDeepStrictEqual(inventory(root), expected)) throw Error('consumer inputs changed');
}

export function fenceProducer(root, source) {
  if (!source || !Array.isArray(source.files) || !source.files.length ||
      !['provisional-working-source', 'committed-release-source'].includes(source.source_mode) ||
      !/^[a-f0-9]{40}(?:[a-f0-9]{24})?$/.test(source.revision) ||
      !/^[a-f0-9]{64}$/.test(source.lock_sha256) ||
      !/^[a-f0-9]{64}$/.test(source.source_inventory_sha256) ||
      digest(source.files) !== source.source_inventory_sha256) throw Error('invalid producer witness');
  const paths = source.files.map(file => file.path).sort();
  if (!isDeepStrictEqual(contents(root), paths)) throw Error('producer file set changed');
  for (const file of source.files) {
    const path = safePath(root, file.path);
    if (!['100644', '100755'].includes(file.mode) || hash(path) !== file.sha256 ||
        (lstatSync(path).mode & 0o111 ? '100755' : '100644') !== file.mode) throw Error('producer source changed');
  }
  if (hash(safePath(root, 'Cargo.lock')) !== source.lock_sha256) throw Error('producer lock changed');
  requireWorkspaceVersion(root, source.package_version);
}

export function requirePackageProvenance(packages, producer) {
  for (const pkg of packages) {
    const original = join(pkg.directory, 'Cargo.toml.orig');
    if (hash(original) !== hash(safePath(producer, `crates/${pkg.name}/Cargo.toml`))) throw Error('package original manifest differs from producer');
    const originalFiles = inventory(safePath(producer, `crates/${pkg.name}`));
    for (const [name, expected] of Object.entries(originalFiles)) {
      if (['Cargo.toml', 'Cargo.lock'].includes(name)) continue;
      if (pkg.files[name] !== expected) throw Error('package omitted or changed producer input');
    }
    for (const [name, actual] of Object.entries(pkg.files)) {
      if (['Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', '.cargo_vcs_info.json'].includes(name)) continue;
      if (hash(safePath(producer, `crates/${pkg.name}/${name}`)) !== actual) throw Error('package source differs from producer');
    }
    // A normalized manifest may name targets inside the package, never external sources.
    const text = readFileSync(join(pkg.directory, 'Cargo.toml'), 'utf8');
    for (const [, path] of text.matchAll(/^\s*(?:path|build)\s*=\s*"([^"]+)"/gm)) {
      if (resolve(pkg.directory, path) !== resolve(safePath(pkg.directory, path))) throw Error('package target escaped');
    }
    if (!('LICENSE' in pkg.files)) throw Error(`package missing LICENSE: ${pkg.name}`);
    if (!(('src/lib.rs' in pkg.files) || ('src/main.rs' in pkg.files))) throw Error(`package missing maintained source: ${pkg.name}`);
  }
}

export function auditAiGraph(metadata, packages, app, appName) {
  if (!metadata || resolve(metadata.workspace_root) !== resolve(app) || !metadata.resolve?.nodes?.length) throw Error('invalid consumer metadata root');
  const resolved = new Set(metadata.resolve.nodes.map(node => node.id));
  const expected = new Map(packages.map(pkg => [pkg.name, pkg]));
  const seen = new Set();
  for (const pkg of metadata.packages.filter(pkg => resolved.has(pkg.id))) {
    if (pkg.name === 'rom' || pkg.name.startsWith('rom-')) {
      const selected = expected.get(pkg.name);
      if (pkg.name === appName && resolve(pkg.manifest_path) === join(resolve(app), 'Cargo.toml')) continue;
      if (!selected || pkg.source !== null || pkg.version !== selected.version ||
          resolve(pkg.manifest_path) !== join(selected.directory, 'Cargo.toml')) throw Error('ROM graph escaped exact extracted package');
      seen.add(pkg.name);
    } else if (pkg.source === null) throw Error('unexpected local dependency');
  }
  if (!seen.has('rom') || (appName !== 'rom' && !seen.has('rom-ai') && appName !== 'rom-ai')) throw Error('missing resolved AI dependencies');
}

export function auditRegistryLock(lock, admittedLocks) {
  function entries(text) {
    return text.split('[[package]]').slice(1).map(section => {
      const field = name => new RegExp(`^${name} = "([^"]+)"`, 'm').exec(section)?.[1] ?? null;
      return { name: field('name'), version: field('version'), source: field('source'), checksum: field('checksum') };
    });
  }
  const admitted = admittedLocks.flatMap(entries);
  for (const item of entries(lock)) {
    if (item.source === null) continue;
    if (!item.source.startsWith('registry+') || !/^[a-f0-9]{64}$/.test(item.checksum ?? '') ||
        !admitted.some(other => isDeepStrictEqual(other, item))) throw Error('unexplained registry drift');
  }
}
