// One canonical asset closure serves assembly and admission.
import { readFileSync } from 'node:fs';
import { files, hash, relativePath, safePath } from './files.mjs';

export function inventory(root) {
  const catalog = JSON.parse(readFileSync(safePath(root, 'skills/rom/workflows.json'), 'utf8'));
  if (catalog.profile_version !== 1) throw Error('unsupported profile');
  if (catalog.platform !== 'linux') throw Error('unsupported workflow platform');
  const pairs = files(root, 'skills/rom').map(path => [path, path]);
  for (const asset of catalog.imports ?? []) pairs.push([asset.source_path, asset.path]);
  pairs.push(['docs/native-extensions.md', 'docs/native-extensions.md'], ['extensions/native-alpha-v1.json', 'extensions/native-alpha-v1.json']);
  for (const path of files(root, 'scripts/skills').filter(path => path.endsWith('.mjs') && !path.endsWith('.test.mjs') && !['assemble.mjs', 'assembly.mjs'].includes(path.split('/').at(-1)))) {
    pairs.push([path, `tools/${path.split('/').at(-1)}`]);
  }
  const assets = pairs.sort((a, b) => a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0).map(([source_path, path]) => ({ path: relativePath(path), source_path, sha256: hash(safePath(root, source_path)) }));
  if (new Set(assets.map(asset => asset.path)).size !== assets.length) throw Error('duplicate asset');
  for (const asset of assets.filter(asset => asset.path.endsWith('/SKILL.md'))) {
    const text = readFileSync(safePath(root, asset.source_path), 'utf8');
    const frontmatter = /^---\nname: ([a-z0-9-]+)\ndescription: (Use when[^\n]+)\n---\n/.exec(text);
    if (!frontmatter || frontmatter[1] !== asset.path.split('/').at(-2) || frontmatter[1].length > 64 || frontmatter[2].length > 1024) throw Error('invalid skill frontmatter');
    if (/\/root\/|\/workspace\/|\.superpowers\//.test(text)) throw Error('nonportable skill reference');
  }
  return { catalog, assets };
}
