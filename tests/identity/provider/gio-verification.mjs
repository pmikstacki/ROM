import { nixSha256 } from './nix-hash.mjs';
export function verifyGioClosure(expected, actual) {
  const records = Array.isArray(actual) ? actual : Object.entries(actual).map(([path, value]) => ({ ...value, path }));
  if (records.length !== expected.records.length || records.some(value => !expected.records.some(item => value.path === `/nix/store/${item.name}`))) throw Error('private closure set mismatch');
  for (const value of records) {
    const item = expected.records.find(item => value.path === `/nix/store/${item.name}`);
    const references = item.fields.References[0].split(' ').filter(Boolean).map(name => `/nix/store/${name}`).sort();
    if (nixSha256(value.narHash) !== item.fields.NarHash[0] || value.narSize !== Number(item.fields.NarSize[0]) || JSON.stringify([...value.references].sort()) !== JSON.stringify(references) || !value.signatures?.some(sig => sig.startsWith('cache.nixos.org-1:'))) throw Error('private closure identity mismatch');
  }
  return records;
}
