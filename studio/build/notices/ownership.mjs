// Classify emitted modules and retain the installed owner's complete notice files.
import { existsSync, lstatSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { noticePath, noticeText, sha256 } from './inventory.mjs';

const noticeName = /^(?:.*[-_.])?(?:licen[sc]e|notice|copying|copyright)(?:[-_.].*)?$/i;
const licenseName = /licen[sc]e|copying/i;
const sourceExtension = /\.(?:[cm]?js|[cm]?ts|jsx|tsx|svelte|css|svg)$/i;
const inside = (root, path) => { const rel = relative(root, path); return rel !== '..' && !rel.startsWith('../') && !rel.startsWith('/'); };
const packageName = /^(?:@[a-z0-9._-]+\/)?[a-z0-9._-]+$/i;
const packageVersion = /^[a-z0-9][a-z0-9.+_-]{0,127}$/i;

function noticeFiles(directory) {
  const found = [], pending = [''];
  let inspected = 0;
  while (pending.length) {
    const prefix = pending.pop();
    for (const entry of readdirSync(join(directory, prefix), { withFileTypes: true })) {
      if (++inspected > 50000) throw Error('runtime notice package inventory exceeds bound');
      const path = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory() && !['node_modules', '.git'].includes(entry.name)) pending.push(path);
      else if (noticeName.test(entry.name) && !sourceExtension.test(entry.name)) {
        if (!entry.isFile() || entry.isSymbolicLink()) throw Error('invalid runtime notice source file');
        found.push(path);
      }
    }
  }
  if (!found.some(path => licenseName.test(path.split('/').at(-1)))) throw Error('missing runtime notice license text');
  return found.sort();
}

export function packageOwner(root, directory, kind = 'npm') {
  const modulesRoot = join(root, 'node_modules');
  let current = resolve(directory);
  while (inside(modulesRoot, current) && current !== modulesRoot) {
    const path = join(current, 'package.json');
    if (existsSync(path)) {
      const pkg = JSON.parse(readFileSync(path, 'utf8'));
      if (pkg.name && pkg.version) {
        if (!packageName.test(pkg.name) || !packageVersion.test(pkg.version) ||
            relative(modulesRoot, current).split('/node_modules/').at(-1) !== pkg.name) throw Error('invalid runtime notice package identity');
        return { directory: current, files: noticeFiles(current), id: `${kind}:${pkg.name}@${pkg.version}`,
          kind, name: pkg.name, version: pkg.version, license_expression: typeof pkg.license === 'string' ? pkg.license : null };
      }
    }
    current = dirname(current);
  }
  throw Error('unclassified runtime notice package');
}

export function moduleOwner(root, originalId) {
  if (originalId === '\0vite/modulepreload-polyfill.js') return { id: 'virtual:vite/modulepreload-polyfill.js',
    owners: ['vite', 'rolldown'].map(name => packageOwner(root, join(root, `node_modules/${name}`), 'tool-runtime')) };
  if (originalId.startsWith('\0')) throw Error('unclassified runtime notice virtual module');
  const query = originalId.indexOf('?'), suffix = query === -1 ? '' : originalId.slice(query);
  const path = resolve(query === -1 ? originalId : originalId.slice(0, query));
  if (inside(join(root, 'node_modules'), path)) return { id: noticeText(noticePath(relative(root, path)) + suffix), owners: [packageOwner(root, dirname(path))] };
  if (!inside(dirname(root), path)) throw Error('unclassified runtime notice external module');
  const id = noticeText((inside(root, path) ? noticePath(relative(root, path)) : `workspace/${noticePath(relative(dirname(root), path))}`) + suffix);
  if (id.startsWith('src/lib/components/ui/') || id.split('?')[0] === 'src/lib/hooks/is-mobile.svelte.ts') {
    return { id, owners: [{ directory: join(root, 'src/lib/components/ui'), files: ['LICENSE.md'], id: 'vendored:shadcn-svelte',
      kind: 'vendored', name: 'shadcn-svelte-vendored', version: 'source-checkout', license_expression: 'MIT' }] };
  }
  return { id, owners: [] };
}

export function retainOwner(owner, assets) {
  const prefix = `notices/${owner.kind}/${owner.name.replace(/[^a-z0-9._-]/gi, '_')}/${owner.version}`;
  const notices = owner.files.map(source => {
    noticePath(source);
    const original = join(owner.directory, source);
    if (!lstatSync(original).isFile() || lstatSync(original).isSymbolicLink()) throw Error('invalid runtime notice source');
    const bytes = readFileSync(original), path = `${prefix}/${source}`;
    if (!bytes.length || bytes.length > 1024 * 1024 || assets.has(path)) throw Error('invalid runtime notice file bounds or collision');
    assets.set(path, bytes);
    return { source, path, sha256: sha256(bytes), bytes: bytes.length };
  });
  noticeText(owner.id);
  return { id: owner.id, kind: owner.kind, name: owner.name, version: owner.version,
    license_expression: owner.license_expression, modules: [], notices };
}
