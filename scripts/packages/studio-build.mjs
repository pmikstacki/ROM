// Build copied locked inputs and retain only the immutable production extraction.
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { copyStudio, studioSource } from './studio-source.mjs';
import { studioAssets, requireStudioAssets } from './studio-assets.mjs';
import { recordedCommand, execute } from '../release-artifacts/command-result.mjs';
import { directoryArchive, extract } from '../release-artifacts/archives.mjs';

export const studioPackageCommands = [['npm', ['ci', '--offline', '--no-audit', '--no-fund']], ['npm', ['run', 'build']]];

export async function prepareStudioAssets(root, stage, artifact, runner = execute) {
  const source = studioSource(root), workspace = mkdtempSync(join(stage, '.studio-work-'));
  const results = [];
  try {
    const copied = join(workspace, 'copied'), directory = copyStudio(root, copied);
    for (const [program, args] of studioPackageCommands) {
      const result = await recordedCommand(program, args, directory, stage, `studio-npm-${results.length + 1}`, runner);
      results.push(result);
      if (result.exit_code !== 0 || result.bounded_abort || result.spawn_failed) throw Error('Studio build command failed');
    }
    try {
      if (!isDeepStrictEqual(studioSource(root), source) || !isDeepStrictEqual(studioSource(copied), source)) throw Error('changed');
    } catch { throw Error('Studio source identity changed during build'); }
    const assets = studioAssets(join(directory, 'dist'));
    cpSync(join(directory, 'dist'), join(workspace, artifact.prefix), { recursive: true });
    directoryArchive(workspace, artifact.prefix, join(stage, artifact.path));
    const extraction = join(workspace, 'extracted');
    mkdirSync(extraction);
    const asset_directory = extract(join(stage, artifact.path), extraction, artifact.prefix);
    requireStudioAssets(asset_directory, assets);
    rmSync(copied, { recursive: true, force: true });
    rmSync(join(workspace, artifact.prefix), { recursive: true, force: true });
    return { source, assets, package_commands: results, asset_directory,
      cleanup: () => rmSync(workspace, { recursive: true, force: true }) };
  } catch (error) {
    writeFileSync(join(stage, 'evidence/studio-package-failure.json'), JSON.stringify({ completed: false, source,
      error: error.message, command_results: results }, null, 2) + '\n', { flag: 'wx' });
    rmSync(workspace, { recursive: true, force: true });
    throw error;
  }
}
