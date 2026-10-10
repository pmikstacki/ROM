// Explicit stylesheet imports survive CSS transformation without a JS module map.
import { existsSync, lstatSync, readFileSync, realpathSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { packageOwner } from './ownership.mjs';
import { installedPackage } from './installed-package.mjs';
import { noticePath, sha256 } from './inventory.mjs';

const specifiers = new Set(['rom-ui/styles', 'shadcn-svelte/tailwind.css', 'tw-animate-css']);
export function importedCssOwners(root) {
  const source = join(root, 'src/app.css');
  if (!existsSync(source)) return [];
  const pending = [{ css: readFileSync(source, 'utf8'), directory: dirname(source) }];
  const seen = new Set(), found = [];
  while (pending.length) {
    const current = pending.pop();
    for (const match of current.css.matchAll(/@import\s+["']([^"']+)["']/g)) {
      const specifier = match[1];
      if (!specifiers.has(specifier)) continue;
      const name = specifier.split('/')[0];
      const directory = installedPackage(root, name, current.directory);
      const owner = packageOwner(root, directory);
      const manifest = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
      const key = specifier === name ? '.' : '.' + specifier.slice(name.length);
      const exported = typeof manifest.exports === 'string' ? manifest.exports : manifest.exports?.[key];
      const target = typeof exported === 'string' ? exported : exported?.style ?? exported?.default;
      if (typeof target !== 'string' || !target.startsWith('./')) throw Error('invalid runtime notice CSS export');
      noticePath(target.slice(2));
      const file = join(directory, target.slice(2));
      if (seen.has(file)) continue;
      if (seen.size >= 16) throw Error('runtime notice CSS import count exceeds bound');
      seen.add(file);
      const id = noticePath(`node_modules/${relative(realpathSync(join(root, "node_modules")), file)}`);
      if (!id.startsWith('node_modules/') || !file.endsWith('.css') || !lstatSync(file).isFile() || lstatSync(file).isSymbolicLink())
        throw Error('invalid runtime notice CSS source');
      const bytes = readFileSync(file);
      if (!bytes.length || bytes.length > 1024 * 1024) throw Error('invalid runtime notice CSS source bounds');
      found.push({ id: 'generated-css:' + id + '#sha256=' + sha256(bytes), owners: [owner] });
      pending.push({ css: bytes.toString('utf8'), directory: dirname(file) });
    }
  }
  return found;
}
