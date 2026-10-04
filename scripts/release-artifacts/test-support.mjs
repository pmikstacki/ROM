// Isolated committed source fixtures; no application gate is claimed here.
import { cpSync, mkdirSync, mkdtempSync, writeFileSync, readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { hash } from '../skills/files.mjs';
import { frontendFixture, fixtureRunner } from '../packages/studio-test-support.mjs';

const source = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const scratch = name => mkdtempSync(join(process.env.ROM_RELEASE_TEST_TMP ?? '/var/tmp', `rom-artifact-${name}-`));
export function fixture() {
  const parent = scratch('test');
  const root = join(parent, 'source');
  mkdirSync(root);
  for (const path of ['scripts/skills', 'skills/rom', 'docs/native-extensions.md', 'extensions/native-alpha-v1.json', 'Cargo.toml', 'Cargo.lock', 'examples/consumer/tests/native_conformance.rs', 'examples/consumer/tests/native_conformance/fixture.rs']) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    cpSync(join(source, path), join(root, path), { recursive: true });
  }
  frontendFixture(root, JSON.parse(readFileSync(join(root, 'extensions/native-alpha-v1.json'))).package_version);
  writeFileSync(join(root, '.gitignore'), '/dist/\n');
  git(root, ['init', '-q']);
  git(root, ['add', '.']);
  git(root, ['-c', 'user.name=ROM fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture']);
  return { root, parent, output: join(parent, 'completed') };
}
export const git = (root, args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', timeout: 5000 });
export const passed = fixtureRunner;
export function mutateManifest(output, mutate) {
  const path = join(output, 'manifest.json');
  const manifest = JSON.parse(readFileSync(path));
  mutate(manifest);
  writeFileSync(path, JSON.stringify(manifest, null, 2) + '\n');
  const sums = join(output, 'SHA256SUMS');
  writeFileSync(sums, readFileSync(sums, 'utf8').replace(/^[0-9a-f]{64}  manifest\.json$/m, `${hash(path)}  manifest.json`));
}
