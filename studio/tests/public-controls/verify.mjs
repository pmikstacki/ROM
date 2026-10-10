// Install the supplied source package into a minimal consumer and verify its public contracts.
import { cpSync, mkdirSync, readFileSync, writeFileSync, lstatSync, realpathSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { isDeepStrictEqual } from 'node:util';
import { reserveOutput, assertLockedGraph, hashBytes, sourceManifest, browserOutcomes, requireAdmission } from './verification.mjs';
import { sourcePrefix, assertStudioArchiveFile, extractStudioArchive } from './archive-admission.mjs';
import { assertPnpmVersions } from './pnpm-verification.mjs';

const studio = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const args = process.argv.slice(2);
let requestedOutput, sourceArchive, sourceDirectory, bootstrap = false, admit = false, pnpm = false;
for (let i = 0; i < args.length; i++) {
  const arg = args[i];
  if (arg === '--source-archive' || arg === '--source-dir') {
    if (!args[i + 1] || args[i + 1].startsWith('--')) throw Error(`${arg} requires a path`);
    if (arg === '--source-archive') sourceArchive = resolve(args[++i]);
    else sourceDirectory = resolve(args[++i]);
  } else if (arg === '--pnpm') pnpm = true;
  else if (arg === '--bootstrap-lock') bootstrap = true;
  else if (arg === '--admit') admit = true;
  else if (arg.startsWith('--') || requestedOutput) throw Error('invalid public-controls verifier arguments');
  else requestedOutput = arg;
}
if (pnpm && admit) throw Error('pnpm is an extraction candidate; the existing release admission profile remains unchanged');
if (sourceArchive && sourceDirectory) throw Error('select one source archive or directory');
if (admit && (bootstrap || process.env.ROM_PUBLIC_CONTROLS_COPY_DEPENDENCIES === '1')) throw Error('release admission forbids dependency bootstrap or copying');
if (process.env.ROM_PUBLIC_CONTROLS_COPY_DEPENDENCIES === '1') throw Error('dependency copying is not supported; use the frozen consumer lock');
const output = reserveOutput(requestedOutput);
const fixture = join(output, 'consumer');
let extracted;
const archive = join(output, 'studio-source.tar.gz');
const sourceMode = sourceArchive ? 'release_artifact' : sourceDirectory ? 'extracted_release_source' : 'authoring_checkout';
const commands = [];
function run(program, argv, cwd, label) {
  const result = spawnSync(program, argv, { cwd, env: process.env, encoding: 'utf8', timeout: 180000, maxBuffer: 8 * 1024 * 1024 });
  const record = { program, args: argv, cwd, exit_code: result.status, signal: result.signal, error: result.error?.message };
  commands.push(record);
  writeFileSync(join(output, label + '.log'), JSON.stringify(record) + '\n' + (result.stdout ?? '') + (result.stderr ?? ''), { flag: 'wx' });
  if (result.status !== 0 || result.error) throw Error(`${label} failed: ${join(output, label + '.log')}`);
  return result;
}
try {
  if (sourceArchive) {
    assertStudioArchiveFile(sourceArchive);
    cpSync(sourceArchive, archive, { errorOnExist: true, force: false });
  } else {
    const source = sourceDirectory ?? studio;
    const stage = join(output, 'source-stage');
    const candidate = join(stage, sourcePrefix);
    mkdirSync(candidate, { recursive: true });
    for (const file of ['package.json', 'package-lock.json', 'src']) cpSync(join(source, file), join(candidate, file), { recursive: true });
    for (const file of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) {
      const licenseSource = existsSync(join(source, file)) ? join(source, file) : join(dirname(source), file);
      cpSync(licenseSource, join(candidate, file));
    }
    sourceManifest(candidate);
    run('tar', ['-czf', archive, '-C', stage, sourcePrefix], stage, 'archive');
  }
  const archiveHash = hashBytes(readFileSync(archive));
  const extractionParent = join(output, 'source-extraction');
  mkdirSync(extractionParent);
  extracted = extractStudioArchive(archive, extractionParent);
  // Keep the local file-dependency path fixed across bootstrap and locked replay.
  cpSync(extracted, join(output, 'extracted'), { recursive: true });
  extracted = join(output, 'extracted');
  const expectedSource = sourceManifest(extracted);
  cpSync(join(studio, 'tests/public-controls/consumer'), fixture, { recursive: true });
  const producer = JSON.parse(readFileSync(join(extracted, 'package.json'), 'utf8'));
  const lockPath = join(fixture, 'package-lock.json');
  if (bootstrap) {
    if (pnpm) run('corepack', ['pnpm@10.30.0', 'import'], fixture, 'bootstrap-pnpm-lock');
    else run('npm', ['install', '--package-lock-only', '--ignore-scripts', '--prefer-offline', '--install-links', '--no-audit', '--no-fund'], fixture, 'bootstrap-lock');
  }
  const pnpmLockPath = join(fixture, 'pnpm-lock.yaml');
  if (pnpm && !existsSync(pnpmLockPath)) throw Error('missing frozen pnpm consumer lock; generate a candidate with --bootstrap-lock');
  const pnpmLockHash = pnpm ? hashBytes(readFileSync(pnpmLockPath)) : null;
  if (!existsSync(lockPath)) throw Error('missing frozen consumer lock; generate a candidate with --bootstrap-lock');
  const frozenLock = JSON.parse(readFileSync(lockPath, 'utf8'));
  const local = frozenLock.packages?.['node_modules/rom-studio'];
  if (local?.version !== producer.version || !isDeepStrictEqual(local.dependencies, producer.dependencies)) throw Error('supplied source dependency metadata differs from frozen consumer lock');
  if (pnpm) run('corepack', ['pnpm@10.30.0', 'install', '--frozen-lockfile', '--ignore-scripts', '--config.node-linker=hoisted', '--config.package-import-method=copy'], fixture, 'install');
  else run('npm', ['ci', '--prefer-offline', '--install-links', '--no-audit', '--no-fund'], fixture, 'install');
  if (pnpm && hashBytes(readFileSync(pnpmLockPath)) !== pnpmLockHash) throw Error('pnpm consumer lock changed during acceptance');
  assertLockedGraph(frozenLock, JSON.parse(readFileSync(lockPath, 'utf8')));
  const installed = join(fixture, 'node_modules/rom-studio');
  if (lstatSync(installed).isSymbolicLink() || !realpathSync(installed).startsWith(realpathSync(fixture) + '/')) throw Error('source package must be installed inside consumer, not linked to checkout');
  const installedSource = sourceManifest(installed);
  if (!isDeepStrictEqual(expectedSource, installedSource)) throw Error('installed source differs from supplied source');
  // Check realized versions, not only the lock that npm ci leaves on disk.
  const realizedVersions = pnpm ? assertPnpmVersions(fixture, frozenLock) : null;
  if (!pnpm) for (const [path, entry] of Object.entries(frozenLock.packages)) {
    if (!path || !entry.version) continue;
    const manifest = join(fixture, path, 'package.json');
    if (!existsSync(manifest) && entry.optional) continue;
    if (!existsSync(manifest) || JSON.parse(readFileSync(manifest, 'utf8')).version !== entry.version) throw Error(`installed dependency graph drift at ${path}`);
  }
  const manager = pnpm ? 'corepack' : 'npm';
  const prefix = pnpm ? ['pnpm@10.30.0'] : [];
  run(manager, [...prefix, 'run', 'check'], fixture, 'type-check');
  run(manager, [...prefix, 'run', 'build'], fixture, 'build');
  let browserReport;
  const browserExecuted = process.env.ROM_PUBLIC_CONTROLS_BROWSER === '1';
  if (browserExecuted) {
    const args = pnpm ? ['pnpm@10.30.0', 'exec', 'playwright', 'test', '--config', 'tests/playwright.config.ts', '--reporter=json'] : ['exec', '--offline', '--', 'playwright', 'test', '--config', 'tests/playwright.config.ts', '--reporter=json'];
    const browser = run(manager, args, fixture, 'browser');
    browserReport = JSON.parse(browser.stdout);
    writeFileSync(join(output, 'browser-results.json'), JSON.stringify(browserReport, null, 2) + '\n', { flag: 'wx' });
  }
  if (pnpm && hashBytes(readFileSync(pnpmLockPath)) !== pnpmLockHash) throw Error('pnpm consumer lock changed during acceptance');
  if (hashBytes(readFileSync(archive)) !== archiveHash) throw Error('source archive changed during acceptance');
  const result = { completed: true, source_mode: sourceMode, source_input: sourceArchive ?? sourceDirectory ?? studio, archive_sha256: archiveHash, installed_package: installed, installed_source_matches: true, source_files: expectedSource, source_lock_sha256: hashBytes(readFileSync(join(extracted, 'package-lock.json'))), consumer_lock_sha256: hashBytes(readFileSync(lockPath)), dependency_bootstrap: pnpm ? (bootstrap ? 'bootstrap_then_pnpm_install' : 'locked_pnpm_install') : (bootstrap ? 'bootstrap_then_npm_ci' : 'locked_npm_ci'), package_manager: pnpm ? 'pnpm@10.30.0' : 'npm', pnpm_lock_sha256: pnpmLockHash, realized_package_versions: realizedVersions, browser_executed: browserExecuted, browser_outcomes: browserOutcomes(browserReport), commands };
  if (admit) requireAdmission(result, browserReport);
  writeFileSync(join(output, 'result.json'), JSON.stringify(result, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ completed: true, admitted: admit, output }));
} catch (error) {
  writeFileSync(join(output, 'failure.json'), JSON.stringify({ completed: false, error: error.message, commands }, null, 2) + '\n', { flag: 'wx' });
  console.error(error.message);
  process.exitCode = 1;
}
