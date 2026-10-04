// Complete immutable production assets; the actual host gate proves execution separately.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { contents } from '../release-artifacts/contents.mjs';
import { digest, hash, relativePath, safePath } from '../skills/files.mjs';
import { isGeneratedStudioPath } from './studio-source.mjs';
import { validateRuntimeNotices } from '../../studio/build/notices/inventory.mjs';

export function studioAssets(root) {
  const paths = contents(root);
  if (!paths.includes('index.html') || paths.some(isGeneratedStudioPath)) throw Error('invalid Studio assets');
  const index = new TextDecoder('utf-8', { fatal: true }).decode(readFileSync(join(root, 'index.html')));
  const references = [...index.matchAll(/<(?:script|link)\b[^>]*\b(?:src|href)="([^"]+)"[^>]*>/g)].map(match => match[1]);
  if (!references.some(path => path.endsWith('.js'))) throw Error('missing Studio application entry');
  for (const reference of references) {
    if (!reference.startsWith('/rom-studio/')) throw Error('external Studio asset reference');
    const path = relativePath(reference.slice('/rom-studio/'.length));
    if (!paths.includes(path)) throw Error('missing Studio asset reference');
    safePath(root, path);
  }
  validateRuntimeNotices(new Map(paths.map(path => [path, readFileSync(safePath(root, path))])));
  const files = Object.fromEntries(paths.map(path => [path, hash(safePath(root, path))]));
  return { base_path: '/rom-studio/', files, sha256: digest(files) };
}

export function requireStudioAssets(root, expected) {
  let actual;
  try { actual = studioAssets(root); }
  catch (error) { throw Error('Studio asset identity mismatch', { cause: error }); }
  if (!isDeepStrictEqual(actual, expected)) throw Error('Studio asset identity mismatch');
  return actual;
}
