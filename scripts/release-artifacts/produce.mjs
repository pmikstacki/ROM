// Admission, fixed acceptance, staged assembly and exclusive local completion.
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { snapshot, fence, toolchain } from './source.mjs';
import { admitOutput, capability, publish } from './publication.mjs';
import { PUBLIC_CONSUMERS_PROFILE, verifyCommands } from './commands.mjs';
import { sourceArchive, skillArchive } from './archives.mjs';
import { verifyExtraction, verifyArtifacts } from './verification.mjs';
import { finalize } from './manifest.mjs';
import { prepareStudioAssets } from '../packages/studio-build.mjs';
import { requireStudioAssets } from '../packages/studio-assets.mjs';
import { studioSource } from '../packages/studio-source.mjs';
import { persistenceError, reportEvidenceFailure } from './failure-diagnostics.mjs';

import { digest, hash } from '../skills/files.mjs';
import { productionProfile, admitProductionRequirements } from './production-inputs.mjs';
import { admitSqliteProfileInput, prepareSqliteProfile, sqliteProfileEnvironment, fenceSqliteProfile } from './sqlite-profile.mjs';
import { executeProductionAcceptance } from './production-acceptance.mjs';
import { requireProductionAcceptance } from './production-verification.mjs';
import { withVerifiedExtraction } from './consumer-source.mjs';
import { runChild } from '../skills/process.mjs';
export async function produce({ root, output, runner, requirements, sqliteProfile }) {
  root = resolve(root);
  const source = snapshot(root);
  if (studioSource(root).package_version !== source.package_version) throw Error('incompatible Studio release version');
  const profile = productionProfile(source.package_version);
  const production = profile === PUBLIC_CONSUMERS_PROFILE;
  const admitted = production ? admitProductionRequirements(requirements ?? (process.env.ROM_RELEASE_REQUIREMENTS_DIRECTORY ? { directory: process.env.ROM_RELEASE_REQUIREMENTS_DIRECTORY, sha256: process.env.ROM_RELEASE_REQUIREMENTS_SHA256 } : undefined), digest(source.files)) : undefined;
  const sqlite = production ? (sqliteProfile ?? admitSqliteProfileInput(process.env.ROM_RELEASE_SQLITE_PROFILE, process.env.ROM_RELEASE_SQLITE_PROFILE_SHA256)) : undefined;
  const name = `rom-${source.package_version}-${source.revision.slice(0, 12)}`;
  output = resolve(output ?? join(production ? join(dirname(root), 'rom-releases') : join(root, 'dist'), name));
  if (production && (output === root || output.startsWith(root + '/'))) throw Error('production output must be outside producer source');
  admitOutput(output);
  const tools = toolchain(root);
  const publication = capability(dirname(output));
  const stage = mkdtempSync(join(dirname(output), '.rom-release-stage-'));
  mkdirSync(join(stage, 'evidence'));
  const results = [];
  let acceptance;
  try {
    const engine = production ? await prepareSqliteProfile(sqlite, join(mkdtempSync(join(dirname(stage), '.rom-sqlite-profile-')), 'engine'), runner ?? runChild) : undefined;
    const selectedRunner = production ? (program, args, options) => (runner ?? runChild)(program, args, { ...options, timeout: options.timeout ?? 3600000, maxBytes: options.maxBytes ?? 32 * 1024 * 1024, env: sqliteProfileEnvironment(options.env, engine.directory) }) : runner;
    await verifyCommands(root, stage, results, selectedRunner);
    fence(root, source);
    const artifacts = { source: { path: `${name}-source.tar.gz`, prefix: name }, skills: { path: `${name}-skills.tar.gz`, prefix: `${name}-skills` }, studio: { path: `${name}-studio.tar.gz`, prefix: `${name}-studio` } };
    const prepared = await prepareStudioAssets(root, stage, artifacts.studio, selectedRunner);
    await verifyCommands(root, stage, results, selectedRunner, prepared.asset_directory);
    requireStudioAssets(prepared.asset_directory, prepared.assets);
    const { cleanup, ...frontend } = prepared;
    cleanup();
    fence(root, source);
    sourceArchive(root, source.revision, name, join(stage, artifacts.source.path));
    skillArchive(root, stage, artifacts.skills.prefix, join(stage, artifacts.skills.path));
    const verification = verifyExtraction(stage, source, artifacts, frontend);
    if (production) {
      fenceSqliteProfile(engine);
      for (const payload of Object.values(artifacts)) payload.sha256 = hash(join(stage, payload.path));
      acceptance = await executeProductionAcceptance({ root, stage, source, artifacts, frontend, assets: prepared.asset_directory, results, tools, requirements: admitted, engine }, selectedRunner);
      await withVerifiedExtraction(stage, source, artifacts, frontend, lease => requireProductionAcceptance(acceptance, lease, stage, root));
      fence(root, source);
    }
    const manifest = finalize(stage, source, tools, results, publication, artifacts, verification, acceptance);
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
