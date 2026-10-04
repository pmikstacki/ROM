// Admission, fixed acceptance, staged assembly and exclusive local completion.
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { snapshot, fence, toolchain } from './source.mjs';
import { admitOutput, capability, publish } from './publication.mjs';
import { verifyCommands } from './commands.mjs';
import { sourceArchive, skillArchive } from './archives.mjs';
import { verifyExtraction, verifyArtifacts } from './verification.mjs';
import { finalize } from './manifest.mjs';
import { prepareStudioAssets } from '../packages/studio-build.mjs';
import { requireStudioAssets } from '../packages/studio-assets.mjs';
import { studioSource } from '../packages/studio-source.mjs';
import { persistenceError, reportEvidenceFailure } from './failure-diagnostics.mjs';

export async function produce({ root, output, runner }) {
  root = resolve(root);
  const source = snapshot(root);
  if (studioSource(root).package_version !== source.package_version) throw Error('incompatible Studio release version');
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
    const artifacts = { source: { path: `${name}-source.tar.gz`, prefix: name }, skills: { path: `${name}-skills.tar.gz`, prefix: `${name}-skills` }, studio: { path: `${name}-studio.tar.gz`, prefix: `${name}-studio` } };
    const prepared = await prepareStudioAssets(root, stage, artifacts.studio, runner);
    await verifyCommands(root, stage, results, runner, prepared.asset_directory);
    requireStudioAssets(prepared.asset_directory, prepared.assets);
    const { cleanup, ...frontend } = prepared;
    cleanup();
    fence(root, source);
    sourceArchive(root, source.revision, name, join(stage, artifacts.source.path));
    skillArchive(root, stage, artifacts.skills.prefix, join(stage, artifacts.skills.path));
    const verification = verifyExtraction(stage, source, artifacts, frontend);
    const manifest = finalize(stage, source, tools, results, publication, artifacts, verification);
    await verifyArtifacts(stage);
    fence(root, source);
    publish(stage, output);
    console.log(`Completed local source artifacts: ${output}`);
    return manifest;
  } catch (error) {
    const failure = { completed: false, source_revision: source.revision, error: error.message, command_results: results,
      ...(error.commandResult ? { failed_command: error.commandResult } : {}) };
    try { writeFileSync(join(stage, 'failure.json'), JSON.stringify(failure, null, 2) + '\n'); }
    catch (storageError) {
      reportEvidenceFailure('release-failure-evidence-failure', {
        source_revision: source.revision, stage, original_error: persistenceError(error),
        persistence_error: persistenceError(storageError), failed_command: error.commandResult ?? null,
        last_command: results.at(-1) ?? null,
      });
    }
    console.error(`Incomplete release evidence retained: ${stage}`);
    throw error;
  }
}
