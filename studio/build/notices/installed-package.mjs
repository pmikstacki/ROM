// Follow the importing package's installed dependency tree, including pnpm's virtual store.
import { existsSync, realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join, relative, resolve } from 'node:path';

export function installedPackage(root, name, importer = root) {
  const modules = realpathSync(join(root, 'node_modules'));
  const search = createRequire(resolve(importer, 'package.json')).resolve.paths(name) ?? [];
  for (const directory of search) {
    const candidate = join(directory, name);
    if (!existsSync(candidate)) continue;
    const installed = realpathSync(candidate), rel = relative(modules, installed);
    if (rel === '..' || rel.startsWith('../') || rel.startsWith('/'))
      throw Error('unclassified runtime notice external module');
    return installed;
  }
  throw Error('unclassified runtime notice package');
}
