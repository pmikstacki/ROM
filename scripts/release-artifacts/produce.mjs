// Admission, fixed acceptance, staged assembly and exclusive local completion.
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { snapshot, fence, toolchain } from './source.mjs';
import { admitOutput, capability, publish } from './publication.mjs';
import { verifyCommands } from './commands.mjs';
import { sourceArchive, skillArchive } from './archives.mjs';
import { verifyExtraction, verifyArtifacts } from './verification.mjs';
import { finalize } from './manifest.mjs';

export async function produce({ root, output, runner }) {
  root = resolve(root);
  const source = snapshot(root);
  const name = `rom-${source.package_version}-${source.revision.slice(0, 12)}`;
  output = resolve(output ?? join(root, 'dist', name));
  admitOutput(output);
  const tools = toolchain(root);
  const publication = capability(dirname(output));
  const stage = mkdtempSync(join(dirname(output), '.rom-release-stage-'));
  mkdirSync(join(stage, 'evidence'));
  const results = [];
  try {
    await verifyCommands(root, stage, results, runner);
    fence(root, source);
    const artifacts = { source: { path: `${name}-source.tar.gz`, prefix: name }, skills: { path: `${name}-skills.tar.gz`, prefix: `${name}-skills` } };
    sourceArchive(root, source.revision, name, join(stage, artifacts.source.path));
    skillArchive(root, stage, artifacts.skills.prefix, join(stage, artifacts.skills.path));
    const verification = verifyExtraction(stage, source, artifacts);
    const manifest = finalize(stage, source, tools, results, publication, artifacts, verification);
    await verifyArtifacts(stage);
    fence(root, source);
    publish(stage, output);
    console.log(`Completed local source artifacts: ${output}`);
    return manifest;
  } catch (error) {
    writeFileSync(join(stage, 'failure.json'), JSON.stringify({ completed: false, source_revision: source.revision, error: error.message, command_results: results }, null, 2) + '\n');
    console.error(`Incomplete release evidence retained: ${stage}`);
    throw error;
  }
}
