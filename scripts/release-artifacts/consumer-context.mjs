// This context describes source-bound command identities, not execution or artifact acceptance.
import { isAbsolute, resolve, relative, sep } from 'node:path';

const keys = ['archive_sha256', 'evidence_root', 'extracted_root', 'host_binary', 'revision', 'source_inventory_sha256', 'studio_sha256', 'tree'];
const sha256 = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const canonical = value => typeof value === 'string' && isAbsolute(value) && resolve(value) === value && value !== '/';
const within = (parent, child) => {
  const path = relative(parent, child);
  return path === '' || (!isAbsolute(path) && path !== '..' && !path.startsWith(`..${sep}`));
};

export function requireConsumerContext(context, producerRoot) {
  if (!context || typeof context !== 'object' || Array.isArray(context) ||
      JSON.stringify(Object.keys(context).sort()) !== JSON.stringify(keys) ||
      !['extracted_root', 'evidence_root', 'host_binary'].every(key => canonical(context[key])) ||
      !['archive_sha256', 'source_inventory_sha256', 'studio_sha256'].every(key => sha256(context[key])) ||
      typeof context.revision !== 'string' || !/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(context.revision) ||
      typeof context.tree !== 'string' || !new RegExp(`^[a-f0-9]{${context.revision.length}}$`).test(context.tree) ||
      within(context.extracted_root, context.evidence_root) || within(context.evidence_root, context.extracted_root) ||
      within(context.extracted_root, context.host_binary) || context.extracted_root === producerRoot) {
    throw Error('invalid consumer context');
  }
  return context;
}
