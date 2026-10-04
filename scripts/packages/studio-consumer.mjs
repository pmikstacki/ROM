// Finite fresh-copy acceptance for production Studio assets and the actual host.
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { gates } from '../release-artifacts/commands.mjs';
import { execute, recordedCommand } from '../release-artifacts/command-result.mjs';
import { hash } from '../skills/files.mjs';
import { prepareStudioAssets } from './studio-build.mjs';
import { requireStudioAssets } from './studio-assets.mjs';
import { studioSource } from './studio-source.mjs';

export async function verifyStudioPackage({ root, output, runner = execute }) {
  root = resolve(root); output = resolve(output);
  mkdirSync(output);
  mkdirSync(join(output, 'evidence'));
  const artifact = { path: 'studio-assets.tar.gz', prefix: 'studio-assets' };
  let prepared, acceptance;
  try {
    prepared = await prepareStudioAssets(root, output, artifact, runner);
    const [program, args] = gates(root, prepared.asset_directory).at(-1);
    acceptance = await recordedCommand(program, args, root, output, 'studio-host', runner);
    if (acceptance.exit_code !== 0 || acceptance.bounded_abort || acceptance.spawn_failed) throw Error('Studio host acceptance failed');
    requireStudioAssets(prepared.asset_directory, prepared.assets);
    if (!isDeepStrictEqual(studioSource(root), prepared.source)) throw Error('Studio source identity changed during host acceptance');
    const { cleanup, ...frontend } = prepared;
    const result = { completed: true, ...frontend, acceptance, artifact: { ...artifact, sha256: hash(join(output, artifact.path)) } };
    writeFileSync(join(output, 'studio-package.json'), JSON.stringify(result, null, 2) + '\n', { flag: 'wx' });
    cleanup();
    return result;
  } catch (error) {
    writeFileSync(join(output, 'studio-package-failure.json'), JSON.stringify({ completed: false, error: error.message,
      source: prepared?.source, acceptance }, null, 2) + '\n', { flag: 'wx' });
    throw error;
  }
}
