#!/usr/bin/env node
// Verify distributable sources without publishing or resolving ROM from crates.io.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, cpSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve, join, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(process.argv[2] ?? join(dirname(fileURLToPath(import.meta.url)), '..'));
const scratch = mkdtempSync(join(tmpdir(), 'rom-packaged-consumer-'));
// The host can retain dependency builds between runs. Each run still creates
// fresh archives, extracts fresh sources and audits the consumer's resolved paths.
const target = process.env.ROM_PACKAGE_TARGET_DIR
  ? resolve(process.env.ROM_PACKAGE_TARGET_DIR) : join(scratch, 'target');
const env = { ...process.env, CARGO_BUILD_JOBS: '2', CARGO_TARGET_DIR: target };
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const lockHash = hash(join(root, 'Cargo.lock'));
const run = (program, args, cwd = root, capture = false) => execFileSync(program, args, {
  cwd, env, encoding: 'utf8', stdio: capture ? ['ignore', 'pipe', 'inherit'] : 'inherit',
});
const metadata = JSON.parse(run('cargo', ['metadata', '--locked', '--format-version', '1', '--no-deps'], root, true));
const packages = metadata.packages.filter(p => p.manifest_path.startsWith(join(root, 'crates') + sep));
if (!packages.length) throw Error('No maintained library packages found');
const consumer = metadata.packages.find(p => p.name === 'rom-consumer');
if (!consumer) throw Error('Public API consumer is missing');

// CLI config patches only unpublished workspace crates; no manifest is rewritten.
const patches = paths => '[patch.crates-io]\n' + packages.map(p =>
  `${JSON.stringify(p.name)} = { path = ${JSON.stringify(paths(p))} }`).join('\n') + '\n';
const sourceConfig = join(scratch, 'source-patches.toml');
writeFileSync(sourceConfig, patches(p => dirname(p.manifest_path)));
for (const p of packages) {
  if (!p.license || !p.description) throw Error(`${p.name}: license/description metadata missing`);
}
run('cargo', ['--config', sourceConfig, 'package', '--no-verify', '--allow-dirty',
  ...packages.flatMap(p => ['-p', p.name])]);

const unpacked = join(scratch, 'unpacked');
mkdirSync(unpacked);
for (const p of packages) {
  const name = `${p.name}-${p.version}`;
  const archive = join(target, 'package', `${name}.crate`);
  const contents = run('tar', ['-tzf', archive], root, true).trim().split('\n');
  if (contents.some(path => !path.startsWith(`${name}/`) || path.split('/').includes('..'))) {
    throw Error(`${p.name}: invalid archive path`);
  }
  run('tar', ['-xzf', archive, '-C', unpacked]);
  if (!existsSync(join(unpacked, name, 'LICENSE'))) throw Error(`${p.name}: packaged MIT license missing`);
  if (readFileSync(join(unpacked, name, 'LICENSE'), 'utf8') !== readFileSync(join(root, 'LICENSE'), 'utf8')) {
    throw Error(`${p.name}: packaged license differs from repository license`);
  }
}

const app = join(scratch, 'consumer');
mkdirSync(join(app, '.cargo'), { recursive: true });
cpSync(join(dirname(consumer.manifest_path), 'src'), join(app, 'src'), { recursive: true });
cpSync(join(dirname(consumer.manifest_path), 'tests'), join(app, 'tests'), { recursive: true });
const archivePath = p => join(unpacked, `${p.name}-${p.version}`);
writeFileSync(join(app, '.cargo', 'config.toml'), patches(archivePath));
const dependencies = kind => consumer.dependencies.filter(d => d.kind === kind).map(d => {
  const options = [`version = ${JSON.stringify(d.req)}`];
  if (!d.uses_default_features) options.push('default-features = false');
  if (d.features.length) options.push(`features = ${JSON.stringify(d.features)}`);
  if (d.rename) options.push(`package = ${JSON.stringify(d.name)}`);
  return `${JSON.stringify(d.rename ?? d.name)} = { ${options.join(', ')} }`;
});
writeFileSync(join(app, 'Cargo.toml'), `[package]\nname = "rom-consumer"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n${dependencies(null).join('\n')}\n\n[dev-dependencies]\n${dependencies('dev').join('\n')}\n\n[workspace]\n`);
cpSync(join(root, 'Cargo.lock'), join(app, 'Cargo.lock'));
for (const p of packages) {
  run('cargo', ['--config', join(app, '.cargo', 'config.toml'), 'check', '--offline', '--all-features',
    '--manifest-path', join(archivePath(p), 'Cargo.toml')], app);
}
// The optional CLI must also run from its extracted package, not from workspace
// paths or a previously built binary. Other packages are library-only here.
const cli = packages.find(p => p.name === 'rom-cli');
if (cli) {
  const help = run('cargo', ['--config', join(app, '.cargo', 'config.toml'), 'run', '--offline',
    '--manifest-path', join(archivePath(cli), 'Cargo.toml'), '--bin', 'rom', '--', '--help'], app, true);
  if (!help.includes('Usage:') || !help.includes('discover')) {
    throw Error('Packaged CLI did not expose its expected public commands');
  }
}
run('cargo', ['run', '--offline'], app);
run('cargo', ['test', '--offline', '--tests'], app);
const external = JSON.parse(run('cargo', ['metadata', '--locked', '--offline', '--format-version', '1'], app, true));
for (const p of external.packages.filter(p => packages.some(lib => lib.name === p.name))) {
  if (!p.manifest_path.startsWith(unpacked + sep)) throw Error(`${p.name}: consumer escaped packaged sources`);
}
if (hash(join(root, 'Cargo.lock')) !== lockHash) throw Error('Packaging unexpectedly changed repository lockfile');
console.log(`Packaged consumer passed using ${packages.length} archives. Evidence retained: ${scratch}`);
