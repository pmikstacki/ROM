// Shared artifact contract. Original notice bytes are hashed, never rewritten.
import { createHash } from 'node:crypto';

export const noticeInventoryPath = 'third-party-notices.json';
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export function noticePath(path) {
  if (typeof path !== 'string' || path.length > 2048 || !path || path.includes('\\') ||
      path.split('/').some(part => !part || part === '.' || part === '..') || /[\x00-\x1f\x7f]/.test(path)) throw Error('invalid runtime notice path');
  return path;
}
export function noticeText(value) {
  if (typeof value !== 'string' || !value || value.length > 2048 || /[\x00-\x1f\x7f]/.test(value)) throw Error('invalid runtime notice text');
  return value;
}
function record(value, keys) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      Object.keys(value).sort().join('\0') !== [...keys].sort().join('\0')) throw Error('invalid runtime notice record');
}
function boundedArray(value, limit) {
  if (!Array.isArray(value) || value.length > limit) throw Error('invalid runtime notice inventory bounds');
  return value;
}
function requireHash(value) { if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) throw Error('invalid runtime notice hash'); }
function unique(set, value) { if (set.has(value)) throw Error('duplicate runtime notice record'); set.add(value); }

export function validateRuntimeNotices(files) {
  const bytes = files.get(noticeInventoryPath);
  if (!bytes || bytes.byteLength > 4 * 1024 * 1024) throw Error('missing or oversized runtime notice inventory');
  let inventory;
  try { inventory = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)); }
  catch { throw Error('invalid runtime notice inventory JSON'); }
  record(inventory, ['schema_version', 'coverage', 'owners', 'outputs']);
  if (inventory.schema_version !== 1 || inventory.coverage !== 'positive-rendered-javascript-and-generated-css') throw Error('unsupported runtime notice profile');
  const owners = new Map(), expectedNotices = new Set(), ownerModules = new Map();
  for (const owner of boundedArray(inventory.owners, 512)) {
    record(owner, ['id', 'kind', 'name', 'version', 'license_expression', 'modules', 'notices']);
    noticeText(owner.id); noticeText(owner.name); noticeText(owner.version);
    if (!['npm', 'tool-runtime', 'vendored'].includes(owner.kind) ||
        !(owner.license_expression === null || typeof owner.license_expression === 'string' && owner.license_expression.length <= 1024)) throw Error('invalid runtime notice owner');
    if (owners.has(owner.id)) throw Error('duplicate runtime notice owner');
    owners.set(owner.id, owner);
    const modules = new Set();
    for (const id of boundedArray(owner.modules, 10000)) { noticeText(id); unique(modules, id); }
    if (!modules.size) throw Error('unused runtime notice owner');
    ownerModules.set(owner.id, modules);
    const notices = boundedArray(owner.notices, 128);
    if (!notices.length || !notices.some(notice => typeof notice.source === 'string' && /licen[sc]e|copying/i.test(notice.source.split('/').at(-1)))) throw Error('missing runtime notice license text');
    const sources = new Set();
    for (const notice of notices) {
      record(notice, ['source', 'path', 'sha256', 'bytes']);
      noticePath(notice.source); unique(sources, notice.source);
      noticePath(notice.path); requireHash(notice.sha256); unique(expectedNotices, notice.path);
      if (!notice.path.startsWith('notices/') || !Number.isSafeInteger(notice.bytes) || notice.bytes < 1 || notice.bytes > 1024 * 1024) throw Error('invalid runtime notice file');
      const content = files.get(notice.path);
      if (!content || content.byteLength !== notice.bytes || sha256(content) !== notice.sha256) throw Error('missing or changed runtime notice file');
    }
  }
  const outputs = new Set(), referencedModules = new Map([...owners.keys()].map(id => [id, new Set()]));
  for (const output of boundedArray(inventory.outputs, 1024)) {
    record(output, ['file', 'sha256', 'modules']);
    noticePath(output.file); requireHash(output.sha256); unique(outputs, output.file);
    if (!/\.(?:js|mjs|css)$/.test(output.file) || !files.has(output.file) || sha256(files.get(output.file)) !== output.sha256) throw Error('changed runtime notice output');
    const modules = new Set();
    for (const module of boundedArray(output.modules, 10000)) {
      record(module, ['id', 'owners']); noticeText(module.id); unique(modules, module.id);
      const selected = new Set();
      for (const owner of boundedArray(module.owners, 16)) {
        unique(selected, owner);
        if (!owners.has(owner) || !ownerModules.get(owner).has(module.id)) throw Error('missing runtime notice module owner');
        referencedModules.get(owner).add(module.id);
      }
      if ((module.id.startsWith('node_modules/') || module.id.startsWith('virtual:') || module.id.startsWith('generated-css:') || module.id.startsWith('src/lib/components/ui/') || module.id.split('?')[0] === 'src/lib/hooks/is-mobile.svelte.ts') && !selected.size) throw Error('missing runtime notice module owner');
    }
  }
  for (const [owner, modules] of ownerModules) {
    if (modules.size !== referencedModules.get(owner).size) throw Error('unbound runtime notice module');
  }
  for (const path of files.keys()) {
    if (/\.(?:js|mjs|css)$/.test(path) && !outputs.has(path)) throw Error('uncovered runtime notice output');
    if (path.startsWith('notices/') && !expectedNotices.has(path)) throw Error('unlisted runtime notice file');
  }
  return inventory;
}
