//! Profile, feature, asset and supplied-source admission before execution.
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { digest, hash, identity, localLinks, safePath } from './files.mjs';
import { inventory } from './assets.mjs';

export function verify(bundle, root, workflow) {
  if (!['resource', 'native', 'operator', 'release'].includes(workflow)) throw Error('unknown workflow');
  const manifest = JSON.parse(readFileSync(join(bundle, 'bundle.json'), 'utf8'));
  if (manifest.bundle_version !== 1 || manifest.profile_version !== 1) throw Error('unsupported profile');
  if (manifest.platform !== 'linux' || process.platform !== 'linux') throw Error('unsupported workflow platform');
  const selected = manifest.workflows?.[workflow];
  if (!selected || selected.prerequisite !== 'source-checkout') throw Error('invalid workflow prerequisite');
  const profile = JSON.parse(readFileSync(safePath(root, 'extensions/native-alpha-v1.json'), 'utf8'));
  if (profile.profile_version !== 1 || !Array.isArray(profile.features)) throw Error('unsupported source profile');
  if (!Array.isArray(selected.features) || selected.features.some(feature => !profile.features.includes(feature))) throw Error('unsupported feature');
  if (!Array.isArray(manifest.assets) || !manifest.assets.length) throw Error('missing assets');
  const seen = new Set();
  for (const asset of manifest.assets) {
    if (seen.has(asset.path)) throw Error('duplicate asset');
    seen.add(asset.path);
    try {
      if (hash(safePath(bundle, asset.path)) !== asset.sha256 || hash(safePath(root, asset.source_path)) !== asset.sha256) throw Error('changed asset');
    } catch { throw Error('invalid asset path or changed asset'); }
    if (asset.path.endsWith('.md')) localLinks(bundle, asset.path);
  }
  for (const path of [selected.skill, selected.asset, 'tools/verify.mjs', 'extensions/native-alpha-v1.json', 'docs/native-extensions.md']) if (!seen.has(path)) throw Error('missing workflow asset');
  const { catalog, assets } = inventory(root);
  if (digest(assets) !== digest(manifest.assets)) throw Error('asset inventory mismatch');
  if (catalog.profile_version !== 1 || catalog.platform !== manifest.platform || digest(catalog.workflows) !== digest(manifest.workflows)) throw Error('workflow asset catalog mismatch');
  const source = identity(root);
  if (source.sha256 !== manifest.source?.sha256 || digest(manifest.source.files) !== source.sha256 || manifest.source.lock_sha256 !== source.lock_sha256) throw Error('source identity mismatch');
  return { workflow, bundle: resolve(bundle), root: resolve(root), manifest, selected };
}
