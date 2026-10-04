// These explicit imports survive CSS transformation without a JS chunk module map.
import { existsSync, lstatSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { packageOwner } from './ownership.mjs';
import { noticePath, sha256 } from './inventory.mjs';

export function importedCssOwners(root) {
  const source = join(root, 'src/app.css');
  if (!existsSync(source)) return [];
  const css = readFileSync(source, 'utf8');
  return ['shadcn-svelte/tailwind.css', 'tw-animate-css'].flatMap(specifier => {
    const imported = [...css.matchAll(/@import\s+["']([^"']+)["']/g)].some(match => match[1] === specifier);
    if (!imported) return [];
    const name = specifier.split('/')[0], directory = join(root, 'node_modules', name);
    const manifest = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
    const key = specifier === name ? '.' : `.${specifier.slice(name.length)}`;
    const exported = typeof manifest.exports === 'string' ? manifest.exports : manifest.exports?.[key];
    const target = typeof exported === 'string' ? exported : exported?.style ?? exported?.default;
    if (typeof target !== 'string' || !target.startsWith('./')) throw Error('invalid runtime notice CSS export');
    noticePath(target.slice(2));
    const path = join(directory, target.slice(2));
    const id = noticePath(relative(root, path));
    if (!id.startsWith('node_modules/') || !path.endsWith('.css') || !lstatSync(path).isFile() || lstatSync(path).isSymbolicLink())
      throw Error('invalid runtime notice CSS source');
    const bytes = readFileSync(path);
    if (!bytes.length || bytes.length > 1024 * 1024) throw Error('invalid runtime notice CSS source bounds');
    return [{ id: `generated-css:${id}#sha256=${sha256(bytes)}`, owners: [packageOwner(root, dirname(path))] }];
  });
}
