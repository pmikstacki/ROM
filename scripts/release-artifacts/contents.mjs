// Artifact inventories cover every file; source-cache exclusions do not apply.
import { lstatSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { relativePath } from '../skills/files.mjs';

export function contents(root) {
  const result = [];
  function walk(directory) {
    for (const entry of readdirSync(join(root, directory), { withFileTypes: true })) {
      const path = directory ? `${directory}/${entry.name}` : entry.name;
      relativePath(path);
      const status = lstatSync(join(root, path));
      if (status.isDirectory()) walk(path);
      else if (status.isFile()) result.push(path);
      else throw Error('unsupported artifact entry');
    }
  }
  walk('');
  return result.sort();
}
