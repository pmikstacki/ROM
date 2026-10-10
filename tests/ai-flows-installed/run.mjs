// Finite offline acceptance of maintained examples against independently extracted crates.
import { cpSync, existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync, lstatSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { hash, safePath } from '../../scripts/skills/files.mjs';
import { extract, archiveCommit } from '../../scripts/release-artifacts/archives.mjs';
import { inspectPackages, inventory } from '../../scripts/packages/astral-adoption.mjs';
import { command } from '../../scripts/packages/astral-owned-command.mjs';
import { rewriteAiManifest, requireAiResults, requireAiCompiler, validateOptions } from './contracts.mjs';
import { fenceInputs, fenceProducer, requirePackageProvenance, auditAiGraph, auditRegistryLock } from './provenance.mjs';
import { prepareCompileBridge } from './compile-bridge.mjs';
import { extractCrate } from './crate-extraction.mjs';

const json = (path, value) => writeFileSync(path, JSON.stringify(value, null, 2) + '\n', { flag: 'wx' });
const within = (parent, path) => path === parent || path.startsWith(parent + sep);
function regular(path) {
  if (realpathSync(path) !== resolve(path) || !lstatSync(path).isFile() || lstatSync(path).size > 128 * 1024 * 1024) throw Error('invalid or oversized regular archive input');
}
function retainFailure(output, error) {
  json(join(output, 'failure.json'), { completed: false, message: error.message });
}
function prepare(options) {
  const output = resolve(options.output), target = resolve(options.target), sourceRoot = resolve(options.sourceRoot);
  for (const path of [output, target]) {
    if (existsSync(path) || realpathSync(dirname(path)) !== dirname(path)) throw Error('occupied or symbolic output');
  }
  const inputs = [sourceRoot, resolve(options.sourceArchive), ...options.crateArchives.map(item => resolve(item.path))];
  if (realpathSync(sourceRoot) !== sourceRoot) throw Error('symbolic producer root');
  if (within(output, target) || within(target, output) || inputs.some(path => within(path, output) || within(path, target) || within(output, path) || within(target, path))) throw Error('overlapping acceptance inputs/output');
  regular(options.sourceArchive);
  if (hash(options.sourceArchive) !== options.sourceArchiveSha256 ||
      (options.source.source_mode === 'committed-release-source' && archiveCommit(options.sourceArchive) !== options.source.revision)) throw Error('source archive identity mismatch');
  fenceProducer(sourceRoot, options.source);
  for (const item of options.crateArchives) {
    regular(item.path);
    if (!/^[a-f0-9]{64}$/.test(item.sha256) || hash(item.path) !== item.sha256 || !/^rom(?:-[a-z-]+)?-\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/.test(item.prefix)) throw Error('crate archive identity mismatch');
  }
  if (new Set(options.crateArchives.map(item => item.prefix)).size !== options.crateArchives.length) throw Error('duplicate crate archive');
  mkdirSync(output, { mode: 0o700 }); mkdirSync(target, { mode: 0o700 });
  const record = { output, target, sourceRoot, options: structuredClone(options), completed: false };
  json(join(output, 'request.json'), record);
  try {
    const sourceParent = join(output, 'source'); mkdirSync(sourceParent);
    record.producer = extract(options.sourceArchive, sourceParent, options.sourcePrefix);
    fenceProducer(record.producer, options.source);
    const packageRoot = join(output, 'packages'); mkdirSync(packageRoot);
    for (const item of options.crateArchives) extractCrate(item.path, packageRoot, item.prefix);
    record.packages = inspectPackages(packageRoot);
    for (const name of ['rom-ai', 'rom-openrouter', 'rom-redb']) if (!record.packages.some(pkg => pkg.name === name)) throw Error('missing installed AI package');
    if (record.packages.some(pkg => pkg.version !== options.source.package_version)) throw Error('package version differs from producer');
    requirePackageProvenance(record.packages, record.producer);
    const appRoot = join(output, 'consumers'); mkdirSync(appRoot);
    record.consumers = [];
    for (const [path, name] of [['examples/ai-flows', 'rom-ai-flows-consumer'], ['tests/ai-tools-compile', 'rom-ai-tools-compile']]) {
      const original = safePath(record.producer, path), app = join(appRoot, name);
      cpSync(original, app, { recursive: true, errorOnExist: true, force: false });
      writeFileSync(join(app, 'Cargo.toml'), rewriteAiManifest(readFileSync(join(app, 'Cargo.toml'), 'utf8'), record.packages));
      mkdirSync(join(app, '.cargo'));
      writeFileSync(join(app, '.cargo/config.toml'), '[patch.crates-io]\n' + record.packages.map(pkg => `${JSON.stringify(pkg.name)} = { path = ${JSON.stringify(pkg.directory)} }`).join('\n') + '\n');
      record.consumers.push({ app, name, files: inventory(app) });
    }
    record.admittedLocks = [readFileSync(join(record.producer, 'Cargo.lock'), 'utf8'), ...record.packages.filter(pkg => existsSync(join(pkg.directory, 'Cargo.lock'))).map(pkg => readFileSync(join(pkg.directory, 'Cargo.lock'), 'utf8')), ...record.consumers.map(item => readFileSync(join(item.app, 'Cargo.lock'), 'utf8'))];
    json(join(output, 'preparation.json'), record);
    return record;
  } catch (error) { retainFailure(output, error); throw error; }
}

function fence(record) {
  const options = record.options;
  fenceProducer(record.sourceRoot, options.source); fenceProducer(record.producer, options.source);
  if (hash(options.sourceArchive) !== options.sourceArchiveSha256) throw Error('source archive changed');
  for (const item of options.crateArchives) if (hash(item.path) !== item.sha256) throw Error('crate archive changed');
  for (const pkg of record.packages) fenceInputs(pkg.directory, pkg.files);
  for (const item of record.consumers) fenceInputs(item.app, item.files);
}

async function execute(record) {
  const options = record.options, evidence = join(record.output, 'evidence'); mkdirSync(evidence);
  const config = join(record.consumers[0].app, '.cargo/config.toml');
  const run = async (app, args, label, budget, program = 'cargo') => {
    fence(record);
    return command({ app, target: record.target, evidence }, args, label, options[budget], options.outputBytes, program);
  };
  const compiler = requireAiCompiler(await run(record.consumers[0].app, ['-Vv'], 'compiler', 'metadataSeconds', 'rustc'), options.compiler);
  const stages = [...record.consumers.map(item => ({ ...item, manifest: join(item.app, 'Cargo.toml') })),
    ...['rom-ai', 'rom-openrouter'].map(name => {
      const pkg = record.packages.find(pkg => pkg.name === name);
      return { app: pkg.directory, name, manifest: join(pkg.directory, 'Cargo.toml'), files: pkg.files, pkg };
    })];
  const results = [];
  for (const stage of stages) {
    const feature = stage.name === 'rom-openrouter' ? ['--features', 'test-support'] : [];
    const base = ['--config', config];
    const metadataArgs = [...base, 'metadata', '--offline', '--format-version', '1', '--manifest-path', stage.manifest, ...feature];
    // Initial resolution may update only this consumer/package lock. No native work starts before graph admission.
    fence(record);
    const initial = JSON.parse(await command({ app: stage.app, target: record.target, evidence }, metadataArgs, `${stage.name}-resolve`, options.metadataSeconds, options.outputBytes));
    const after = inventory(stage.app), expected = { ...stage.files }; delete after['Cargo.lock']; delete expected['Cargo.lock'];
    if (!isDeepStrictEqual(after, expected)) throw Error('resolution changed source inputs');
    const lock = readFileSync(join(stage.app, 'Cargo.lock'), 'utf8'); auditRegistryLock(lock, record.admittedLocks);
    stage.files['Cargo.lock'] = hash(join(stage.app, 'Cargo.lock'));
    auditAiGraph(initial, record.packages, stage.app, stage.name);
    const frozen = JSON.parse(await run(stage.app, [...metadataArgs, '--locked'], `${stage.name}-metadata`, 'metadataSeconds'));
    auditAiGraph(frozen, record.packages, stage.app, stage.name);
    if (stage.name !== 'rom-ai-tools-compile') {
      const text = await run(stage.app, [...base, 'test', '--offline', '--locked', '--manifest-path', stage.manifest, ...feature], `${stage.name}-tests`, 'testSeconds');
      results.push({ name: stage.name, ...requireAiResults(text, stage.name === 'rom-ai-flows-consumer' ? 'flows' : stage.name === 'rom-openrouter' ? 'adapter' : 'ai') });
    } else {
      // The existing public fixture checks two positive APIs and exact E0599 at its original source span.
      // Execute its unchanged script in a mirrored path so its relative manifest still selects this isolated copy.
      const bridge = prepareCompileBridge(stage.app, join(record.output, 'compile-root'));
      const stdout = await run(bridge.app, [bridge.checker], 'public-readonly-compile', 'compileSeconds', process.execPath);
      fenceInputs(bridge.app, bridge.inputs);
      if (!stdout.includes('execute correctly rejected with E0599 at source line 4')) throw Error('missing negative compile acceptance');
      results.push({ name: stage.name, positive_apis: true, negative_execute: 'E0599:cannot_execute.rs:4', mirror_inputs: bridge.inputs });
    }
    fence(record);
    const finalMetadata = JSON.parse(await run(stage.app, [...metadataArgs, '--locked'], `${stage.name}-final-metadata`, 'metadataSeconds'));
    auditAiGraph(finalMetadata, record.packages, stage.app, stage.name);
  }
  fence(record);
  return { completed: true, source: options.source, source_archive_sha256: options.sourceArchiveSha256,
    crate_archives: options.crateArchives, compiler, target: record.target, results,
    packages: record.packages, consumers: record.consumers, offline: true, cargo_jobs: 2, incremental: false,
    publication: false, acceptance_scope: options.source.source_mode,
    limitations: 'Native installed-consumer acceptance only. Provisional source is not a clean release artifact. Browser progress, original application adoption and human ergonomics remain separate.' };
}

export async function runAiConsumer(options) {
  validateOptions(options);
  if (!options.sourceRoot || !options.sourceArchive || !options.output || !options.target || !options.source ||
      !/^[a-f0-9]{64}$/.test(options.sourceArchiveSha256 ?? '') || !Array.isArray(options.crateArchives) || !options.crateArchives.length || options.crateArchives.length > 32 ||
      !/^[A-Za-z0-9][A-Za-z0-9._+-]*$/.test(options.sourcePrefix ?? '')) throw Error('explicit producer/package inputs required');
  const record = prepare(options), priorOffline = process.env.CARGO_NET_OFFLINE;
  process.env.CARGO_NET_OFFLINE = 'true';
  try {
    const accepted = await execute(record);
    json(join(record.output, 'acceptance.json'), accepted);
    return accepted;
  } catch (error) { retainFailure(record.output, error); throw error; }
  finally {
    try {
      json(join(record.output, 'final-input-observation.json'), {
        producer: inventory(record.sourceRoot), extracted_producer: inventory(record.producer),
        source_archive_sha256: hash(record.options.sourceArchive),
        crate_archives: record.options.crateArchives.map(item => ({ path: item.path, sha256: hash(item.path) })),
        packages: record.packages.map(pkg => ({ name: pkg.name, expected: pkg.files, observed: inventory(pkg.directory) })),
        consumers: record.consumers.map(item => ({ name: item.name, expected: item.files, observed: inventory(item.app) })),
      });
    } catch (error) {
      // Secondary evidence failure must not replace the primary native result.
      process.stderr.write(`AI final input observation unavailable: ${error.code ?? error.message}\n`);
    } finally {
      if (priorOffline === undefined) delete process.env.CARGO_NET_OFFLINE; else process.env.CARGO_NET_OFFLINE = priorOffline;
    }
  }
}
