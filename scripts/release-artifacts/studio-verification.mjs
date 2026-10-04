// Bind extracted production assets and copied npm inputs to the recorded acceptance.
import { isDeepStrictEqual } from 'node:util';
import { studioSource } from '../packages/studio-source.mjs';
import { requireStudioAssets } from '../packages/studio-assets.mjs';
import { studioPackageCommands } from '../packages/studio-build.mjs';

export function verifyStudioSource(extracted, source, frontend) {
  const actual = studioSource(extracted);
  if (!isDeepStrictEqual(actual, frontend?.source) || actual.package_version !== source.package_version) throw Error('archive Studio source identity mismatch');
  if (!Array.isArray(frontend.package_commands) || frontend.package_commands.length !== studioPackageCommands.length ||
      frontend.package_commands.some((result, index) => result.exit_code !== 0 || result.bounded_abort !== false || result.spawn_failed !== false ||
        JSON.stringify([result.program, result.args]) !== JSON.stringify(studioPackageCommands[index]))) throw Error('incomplete Studio package commands');
  return actual;
}
export function verifyStudioExtraction(directory, source, frontend) {
  return { ...frontend, source, assets: requireStudioAssets(directory, frontend.assets) };
}
