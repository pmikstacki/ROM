// Independent author acceptance from a source-bound Studio archive, never workspace imports.
import { cpSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, symlinkSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { runChild } from '../skills/process.mjs';
import { isDeepStrictEqual } from 'node:util';
import { copyStudio, studioSource } from '../packages/studio-source.mjs';
import { directoryArchive, extract } from '../release-artifacts/archives.mjs';
import { applicationInputs } from '../packages/application.mjs';
import { hash } from '../skills/files.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const output = process.argv[2] ? resolve(process.argv[2]) : mkdtempSync('/var/tmp/rom-studio-author-');
mkdirSync(output, { recursive: true });
const source = studioSource(root);
const author = join(root, 'examples/studio-consumer');
const authorPaths = ['src', 'tests', 'index.html', 'tsconfig.json', 'svelte.config.js', 'vite.config.ts', 'README.md'];
const authorFiles = applicationInputs(author, authorPaths);
for (const name of Object.keys(authorFiles).filter(name => name.startsWith('src/'))) {
  const text = readFileSync(join(author, name), 'utf8');
  if (/from\s+["'][^"']*(?:studio\/src|\/lib\/)[^"']*["']/.test(text)) throw Error(`private consumer import: ${name}`);
}
const commands = [];
const environment = { ...process.env };
async function run(program, args, cwd, label, env = environment) {
  const log = join(output, `${label}.log`);
  let result, spawnFailed = false;
  try { result = await runChild(program, args, { cwd, env, timeout: 180000, maxBytes: 8 * 1024 * 1024 }); }
  catch (error) { spawnFailed = true; result = { code: null, timedOut: false, stdout: '', stderr: error.message }; }
  writeFileSync(log, JSON.stringify({ program, args, cwd, code: result.code, timedOut: result.timedOut, spawn_failed: spawnFailed }) + '\n' + result.stdout + result.stderr);
  commands.push({ program, args, cwd, exit_code: result.code, timed_out: result.timedOut, spawn_failed: spawnFailed, log });
  if (spawnFailed || result.timedOut || result.code !== 0) throw Error(`${label} failed; inspect ${log}`);
  return result.stdout.trim();
}
let archive, directory, binary, binaryHash;
try {
  binary = environment.ROM_STUDIO_DEMO_BINARY;
  if (!binary) throw Error("ROM_STUDIO_DEMO_BINARY must name an accepted immutable binary");
  binary = resolve(binary);
  binaryHash = hash(binary);
  const copied = join(output, 'copied');
  copyStudio(root, copied);
  archive = join(output, 'studio-source.tar.gz');
  directoryArchive(copied, 'studio', archive);
  const extracted = join(output, 'extracted');
  mkdirSync(extracted);
  const sdk = extract(archive, extracted, 'studio');
  if (!isDeepStrictEqual(studioSource(extracted), source)) throw Error('extracted Studio identity mismatch');
  directory = join(extracted, 'consumer');
  mkdirSync(directory);
  for (const path of authorPaths) cpSync(join(author, path), join(directory, path), { recursive: true });
  if (!isDeepStrictEqual(applicationInputs(directory, authorPaths), authorFiles)) throw Error('copied author identity mismatch');
  const pkg = JSON.parse(readFileSync(join(sdk, 'package.json'), 'utf8'));
  const lock = JSON.parse(readFileSync(join(sdk, 'package-lock.json'), 'utf8'));
  pkg.name = 'rom-studio-author-example';
  pkg.scripts = { check: 'svelte-check --tsconfig ./tsconfig.json', build: 'vite build' };
  lock.name = pkg.name;
  lock.packages[''].name = pkg.name;
  writeFileSync(join(directory, 'package.json'), JSON.stringify(pkg, null, 2) + '\n');
  writeFileSync(join(directory, 'package-lock.json'), JSON.stringify(lock, null, 2) + '\n');
  await run('npm', ['ci', '--offline', '--no-audit', '--no-fund'], directory, 'npm-ci');
  // The SDK and author resolve one copied dependency graph. This points only inside extraction.
  symlinkSync('../consumer/node_modules', join(sdk, 'node_modules'));
  await run('npm', ['run', 'check'], directory, 'type-check');
  await run('npm', ['run', 'build'], directory, 'build');
  const provider = join(extracted, 'demo/provider-fixture');
  mkdirSync(dirname(provider), { recursive: true });
  cpSync(join(root, 'demo/provider-fixture'), provider, { recursive: true, filter: path => !path.split('/').includes('node_modules') });
  await run('npm', ['ci', '--offline', '--no-audit', '--no-fund'], provider, 'provider-npm-ci');
  const webkit = environment.ROM_WEBKIT_EXECUTABLE ?? await run(join(root, 'scripts/studio-browser-runtime'), [], root, 'webkit-runtime');
  if (!webkit) throw Error('WebKit runtime is required for independent author acceptance');
  await run('npm', ['exec', '--offline', '--', 'playwright', 'test', '--config', 'tests/playwright.config.ts'], directory, 'browser', {
    ...environment, ROM_STUDIO_ASSETS: join(directory, 'dist'), ROM_STUDIO_DEMO_BINARY: binary, ROM_WEBKIT_EXECUTABLE: webkit,
  });
  if (hash(binary) !== binaryHash) throw Error('native binary changed during acceptance');
  if (!isDeepStrictEqual(studioSource(root), source) || !isDeepStrictEqual(studioSource(extracted), source)) throw Error('Studio source changed during acceptance');
  if (!isDeepStrictEqual(applicationInputs(author, authorPaths), authorFiles)) throw Error('author input changed during acceptance');
  if (!isDeepStrictEqual(applicationInputs(directory, authorPaths), authorFiles)) throw Error('copied author input changed during acceptance');
  writeFileSync(join(output, 'result.json'), JSON.stringify({ completed: true, source, author_files: authorFiles, archive_sha256: hash(archive), binary_path: binary, binary_sha256: binaryHash, directory, commands }, null, 2) + '\n');
  console.log(JSON.stringify({ completed: true, output, directory, source_sha256: source.sha256, binary_sha256: binaryHash }));
} catch (error) {
  writeFileSync(join(output, 'failure.json'), JSON.stringify({ completed: false, error: error.message, source, author_files: authorFiles, commands }, null, 2) + '\n');
  console.error(error.message);
  process.exitCode = 1;
}
