#!/usr/bin/env node
// Verify distributable sources without publishing or resolving ROM from crates.io.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, resolve, join, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { applicationInputs, copyApplication, auditPackagePaths } from './packages/application.mjs';

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
  maxBuffer: 32 * 1024 * 1024,
});
run(process.execPath, ['--test', ...readdirSync(join(root,'scripts/packages')).filter(name=>name.endsWith('.test.mjs')).sort().map(name=>join(root,'scripts/packages',name))]);
const metadata = JSON.parse(run('cargo', ['metadata', '--locked', '--format-version', '1', '--no-deps'], root, true));
const packages = metadata.packages.filter(p => p.manifest_path.startsWith(join(root, 'crates') + sep));
if (!packages.length) throw Error('No maintained library packages found');
const consumer = metadata.packages.find(p => p.name === 'rom-consumer');
if (!consumer) throw Error('Public API consumer is missing');
const reference = metadata.packages.find(p => p.name === 'rom-demo');
if (!reference) throw Error('Reference application is missing');

// CLI config patches only unpublished workspace crates; no manifest is rewritten.
const patches = paths => '[patch.crates-io]\n' + packages.map(p =>
  `${JSON.stringify(p.name)} = { path = ${JSON.stringify(paths(p))} }`).join('\n') + '\n';
const sourceConfig = join(scratch, 'source-patches.toml');
writeFileSync(sourceConfig, patches(p => dirname(p.manifest_path)));
for (const p of packages) {
  if (!p.license || !p.description) throw Error(`${p.name}: license/description metadata missing`);
}
run('cargo', ['--config', sourceConfig, 'package', '--offline', '--no-verify', '--allow-dirty',
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

const archivePath = p => join(unpacked, `${p.name}-${p.version}`);
const app = copyApplication(root, consumer, scratch, patches(archivePath));
const consumerInputs = applicationInputs(scratch, [basename(app)]);
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
auditPackagePaths(external, packages, unpacked);
const demo = copyApplication(root, reference, scratch, patches(archivePath), {sharedSupport:true, providerFixture:true});
const demoInputs = applicationInputs(scratch, [basename(demo), 'tests']);
run('cargo', ['test', '--offline', '--all-features'], demo);
const application = JSON.parse(run('cargo', ['metadata', '--locked', '--offline', '--all-features', '--format-version', '1'], demo, true));
auditPackagePaths(application, packages, unpacked);
if (hash(join(root, 'Cargo.lock')) !== lockHash) throw Error('Packaging unexpectedly changed repository lockfile');
writeFileSync(join(scratch,'acceptance.json'), JSON.stringify({
  source_lock_sha256:lockHash,
  libraries:packages.map(p=>({name:p.name,version:p.version,archive_sha256:hash(join(target,'package',`${p.name}-${p.version}.crate`))})),
  applications:[
    {name:consumer.name,path:app,inputs_sha256:consumerInputs,resolved_lock_sha256:hash(join(app,'Cargo.lock'))},
    {name:reference.name,path:demo,inputs_sha256:demoInputs,resolved_lock_sha256:hash(join(demo,'Cargo.lock'))},
  ],
  verified_dependency_root:unpacked,
  reference_tests:'cargo test --offline --all-features',
  real_provider:'Separate source ./demo/verify-provider gate; not rerun against this copied application',
  publication:false,
},null,2)+'\n');
console.log(`Packaged consumer and reference application passed using ${packages.length} archives. Evidence retained: ${scratch}`);
