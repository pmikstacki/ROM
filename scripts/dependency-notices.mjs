// Narrow, version-pinned additions to the Cargo archive notice scan. No network I/O.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const defaultRoot = fileURLToPath(new URL('../licenses/dependency-supplements/', import.meta.url));
export function additionalNotices(pkg, supplementRoot = defaultRoot) {
  const manifest = JSON.parse(readFileSync(join(supplementRoot, 'manifest.json'), 'utf8'));
  const matches = entry => entry.name === pkg.name && entry.version === pkg.version;
  const archiveRoot = dirname(pkg.manifest_path);
  const result = manifest.archiveFiles.filter(matches).flatMap(entry => entry.files.map(file => ({
    label: file,
    text: readFileSync(join(archiveRoot, file), 'utf8'),
    provenance: 'Additional notice preserved in the locked Cargo archive.',
  })));
  for (const entry of manifest.supplements.filter(matches)) {
    const vcs = JSON.parse(readFileSync(join(archiveRoot, '.cargo_vcs_info.json'), 'utf8'));
    if (vcs.git.sha1 !== entry.revision) throw Error(`${pkg.name}: supplement source revision mismatch`);
    if (!entry.url.includes(`/${entry.revision}/`)) throw Error(`${pkg.name}: supplement URL is not revision-pinned`);
    const bytes = readFileSync(join(supplementRoot, entry.file));
    const hash = createHash('sha256').update(bytes).digest('hex');
    if (hash !== entry.sha256) throw Error(`${pkg.name}: supplement checksum mismatch`);
    result.push({
      label: `Upstream ${entry.license} supplement`,
      text: bytes.toString('utf8'),
      provenance: `${entry.reason}. Source: ${entry.url}. Cargo source revision: \`${entry.revision}\`. SHA-256: \`${hash}\`. Retained locally; generation performs no download. This reproduces the ${entry.license} alternative without changing the crate's declared license expression.`,
    });
  }
  return result;
}
