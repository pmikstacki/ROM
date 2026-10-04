// Only emitted module metadata determines JavaScript ownership, not npm's whole graph.
import { join, resolve } from 'node:path';
import { moduleOwner, packageOwner, retainOwner } from './ownership.mjs';
import { noticeInventoryPath, noticePath, sha256, validateRuntimeNotices } from './inventory.mjs';

export function collectRuntimeNotices(directory, bundle) {
  const root = resolve(directory), assets = new Map(), owners = new Map(), outputs = [], outputBytes = new Map();
  const attach = (module, output) => {
    for (const owner of module.owners) {
      if (!owners.has(owner.id)) owners.set(owner.id, retainOwner(owner, assets));
      const record = owners.get(owner.id);
      if (!record.modules.includes(module.id)) record.modules.push(module.id);
    }
    output.modules.push({ id: module.id, owners: module.owners.map(owner => owner.id).sort() });
  };
  for (const chunk of Object.values(bundle)) {
    if (!/\.(?:js|mjs|css)$/.test(chunk.fileName)) continue;
    if (/\.(?:js|mjs)$/.test(chunk.fileName) && chunk.type !== 'chunk') throw Error('unclassified runtime notice JavaScript provenance');
    const code = chunk.type === 'chunk' ? Buffer.from(chunk.code) : Buffer.from(chunk.source);
    const output = { file: noticePath(chunk.fileName), sha256: sha256(code), modules: [] };
    outputBytes.set(output.file, code);
    if (chunk.type === 'chunk') {
      for (const [id, module] of Object.entries(chunk.modules)) {
        if (!Number.isSafeInteger(module.renderedLength) || module.renderedLength < 0) throw Error('invalid runtime notice module length');
        if (module.renderedLength > 0) attach(moduleOwner(root, id), output);
      }
    } else {
      const banner = code.toString().match(/\/\*! tailwindcss v([^\s]+) \|/);
      if (banner) {
        const owner = packageOwner(root, join(root, 'node_modules/tailwindcss'));
        if (banner[1] !== owner.version) throw Error('runtime notice stylesheet version mismatch');
        attach({ id: 'generated-css:tailwindcss', owners: [owner] }, output);
      }
    }
    output.modules.sort((a, b) => a.id.localeCompare(b.id, 'en'));
    outputs.push(output);
  }
  const inventory = { schema_version: 1, coverage: 'positive-rendered-javascript-and-generated-css',
    owners: [...owners.values()].sort((a, b) => a.name.localeCompare(b.name, 'en')),
    outputs: outputs.sort((a, b) => a.file.localeCompare(b.file, 'en')) };
  for (const owner of inventory.owners) owner.modules.sort();
  assets.set(noticeInventoryPath, Buffer.from(JSON.stringify(inventory, null, 2) + '\n'));
  validateRuntimeNotices(new Map([...assets, ...outputBytes]));
  return assets;
}
