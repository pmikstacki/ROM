//! Exclusive deterministic distribution from maintained sources.
import { mkdirSync, copyFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { identity, localLinks, safePath } from './files.mjs';
import { inventory } from './assets.mjs';

export function assemble(root, output) {
  if (existsSync(output)) throw Error('output exists');
  const { catalog, assets } = inventory(root);
  mkdirSync(output);
  for (const { source_path, path } of assets) {
    mkdirSync(dirname(join(output, path)), { recursive: true });
    copyFileSync(safePath(root, source_path), join(output, path));
  }
  for (const { path } of assets.filter(asset => asset.path.endsWith('.md'))) localLinks(output, path);
  const manifest = { bundle_version: 1, profile_version: 1, platform: catalog.platform, workflows: catalog.workflows, source: identity(root), assets };
  writeFileSync(join(output, 'bundle.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  return manifest;
}
