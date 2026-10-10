import { lstatSync, readdirSync, realpathSync } from 'node:fs';
import { nativeTlsPrivatePath } from './native-tls-io.mjs';
// This is a measured allocation guard, not an aggregate filesystem quota.
export function nativeTlsAllocation(directory, maximum = 1073741824) {
  nativeTlsPrivatePath(directory);
  return nativeTlsOwnedDirectoryAllocation(directory, maximum);
}
export function nativeTlsOwnedDirectoryAllocation(directory, maximum = 1073741824) {
  if (!Number.isSafeInteger(maximum) || maximum < 1 || maximum > 1073741824) throw Error('bounded native TLS planned budget required');
  if (realpathSync(directory) !== directory) throw Error('canonical owned allocation root required');
  let count = 0, allocated = 0, logical = 0;
  function walk(path, depth) {
    if (++count > 10000 || depth > 24) throw Error('owned fixture inventory exceeded bound');
    const value = lstatSync(path);
    if (value.isSymbolicLink() || (!value.isFile() && !value.isDirectory())) throw Error('owned fixture contains unexpected file type');
    allocated += value.blocks * 512; logical += value.isFile() ? value.size : 0;
    if (allocated > maximum || logical > maximum) throw Error('owned native TLS allocation exceeded planned budget');
    if (value.isDirectory()) for (const name of readdirSync(path)) walk(`${path}/${name}`, depth + 1);
  }
  walk(directory, 0);
  return { files_and_directories: count, allocated_bytes: allocated, logical_bytes: logical, planned_budget_bytes: maximum, aggregate_hard_quota: false };
}

export function requireNativeTlsCombinedAllocation(caseAllocation, temporaryAllocation, maximum = 1073741824) {
  if (!Number.isSafeInteger(maximum) || maximum < 1 || maximum > 1073741824) throw Error('bounded combined native TLS allocation required');
  for (const allocation of [caseAllocation, temporaryAllocation]) for (const key of ['allocated_bytes', 'logical_bytes']) if (!Number.isSafeInteger(allocation?.[key]) || allocation[key] < 0) throw Error('bounded combined native TLS allocation required');
  const allocated = caseAllocation.allocated_bytes + temporaryAllocation.allocated_bytes, logical = caseAllocation.logical_bytes + temporaryAllocation.logical_bytes;
  if (allocated > maximum || logical > maximum) throw Error('combined native TLS allocation exceeded planned budget');
  return { allocated_bytes: allocated, logical_bytes: logical, planned_budget_bytes: maximum, aggregate_hard_quota: false };
}
