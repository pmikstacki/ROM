// Verify payload digests, exact committed files and extracted-source bundle admission.
import { mkdtempSync, readFileSync, rmSync, lstatSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { isDeepStrictEqual } from 'node:util';
import { hash, identity, relativePath, safePath } from '../skills/files.mjs';
import { verify } from '../skills/admission.mjs';
import { extract, archiveCommit } from './archives.mjs';
import { contents } from './contents.mjs';
import { requireCompleteGate } from './commands.mjs';
import { extractedTree } from './tree.mjs';

export function verifyExtraction(directory, source, artifacts) {
  if (archiveCommit(join(directory, artifacts.source.path)) !== source.revision) throw Error('archive commit identity mismatch');
  const scratch = mkdtempSync(join(tmpdir(), 'rom-release-extraction-'));
  try {
    const extracted = extract(join(directory, artifacts.source.path), scratch, artifacts.source.prefix);
    const profile = JSON.parse(readFileSync(join(extracted, 'extensions/native-alpha-v1.json'), 'utf8'));
    if (!isDeepStrictEqual(source.profile, profile) || source.package_version !== profile.package_version) throw Error('archive release profile identity mismatch');
    const bundle = extract(join(directory, artifacts.skills.path), scratch, artifacts.skills.prefix);
    const actual = contents(extracted);
    if (JSON.stringify(actual) !== JSON.stringify(source.files.map(file => file.path).sort())) throw Error('archive source file identity mismatch');
    for (const file of source.files) {
      const path = join(extracted, file.path);
      if (hash(path) !== file.sha256 || (lstatSync(path).mode & 0o111 ? '100755' : '100644') !== file.mode) throw Error('archive source identity mismatch');
    }
    if (extractedTree(extracted, actual, scratch, source.object_format) !== source.tree) throw Error('archive source tree identity mismatch');
    const selected = identity(extracted);
    if (!isDeepStrictEqual(selected, source.selected) || selected.lock_sha256 !== source.lock_sha256) throw Error('archive selected source identity mismatch');
    const workflows = ['resource', 'native', 'operator', 'release'];
    for (const workflow of workflows) verify(bundle, extracted, workflow);
    return { source_identity: selected.sha256, lock_sha256: selected.lock_sha256, workflows, examples_executed: false };
  } finally { rmSync(scratch, { recursive: true, force: true }); }
}
export async function verifyArtifacts(directory) {
  const manifest = JSON.parse(readFileSync(join(directory, 'manifest.json'), 'utf8'));
  if (manifest.manifest_version !== 1 || manifest.complete !== true || manifest.publication_enabled !== false || manifest.distribution !== 'source-only') throw Error('unsupported artifact manifest');
  requireCompleteGate(manifest.command_results);
  const expected = new Map();
  for (const line of readFileSync(join(directory, 'SHA256SUMS'), 'utf8').trim().split('\n')) {
    const match = /^([0-9a-f]{64})  (.+)$/.exec(line);
    if (!match || expected.has(match[2])) throw Error('invalid checksums');
    relativePath(match[2]);
    expected.set(match[2], match[1]);
    if (hash(safePath(directory, match[2])) !== match[1]) throw Error('artifact checksum mismatch');
  }
  const paths = contents(directory).filter(path => path !== 'SHA256SUMS');
  if (JSON.stringify(paths) !== JSON.stringify([...expected.keys()].sort()) || !expected.has('manifest.json')) throw Error('incomplete artifact checksums');
  for (const payload of Object.values(manifest.artifacts)) {
    relativePath(payload.path);
    if (expected.get(payload.path) !== payload.sha256) throw Error('manifest artifact checksum mismatch');
  }
  const verified = verifyExtraction(directory, manifest.source, manifest.artifacts);
  if (!isDeepStrictEqual(verified, manifest.archive_verification)) throw Error('archive verification mismatch');
  return manifest;
}
