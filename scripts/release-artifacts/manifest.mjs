// Versioned local distribution evidence, with no checksum self-reference.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { hash } from '../skills/files.mjs';
import { contents } from './contents.mjs';
import { STUDIO_PROFILE, PUBLIC_CONSUMERS_PROFILE } from './commands.mjs';

export function finalize(stage, source, toolchain, commandResults, publication, artifacts, verification, production) {
  for (const payload of Object.values(artifacts)) payload.sha256 = hash(join(stage, payload.path));
  const manifest = { manifest_version: production ? 3 : 2, verification_profile: production ? PUBLIC_CONSUMERS_PROFILE : STUDIO_PROFILE, complete: true, distribution: 'source-and-studio-assets', publication_enabled: false, created_at: new Date().toISOString(), source, toolchain, command_results: commandResults, host_publication: publication, artifacts, archive_verification: verification, ...(production ? { production_acceptance: production } : {}) };
  writeFileSync(join(stage, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  const sums = contents(stage).map(path => `${hash(join(stage, path))}  ${path}`).join('\n') + '\n';
  writeFileSync(join(stage, 'SHA256SUMS'), sums, { flag: 'wx' });
  return manifest;
}
