// Executes fixed consumer commands. Passing commands alone do not admit a release.
import { mkdirSync, lstatSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join, resolve, isAbsolute, dirname, relative, sep } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { hash, safePath } from '../skills/files.mjs';
import { verifyCommands, requireCompleteGate, STUDIO_PROFILE, PUBLIC_CONSUMERS_PROFILE } from './commands.mjs';
import { runChild } from '../skills/process.mjs';
import { CONSUMER_TIMEOUT_MS, COMMAND_LOG_BYTES } from './command-limits.mjs';
import { requireConsumerContext } from './consumer-context.mjs';
import { requireNativePreparation } from './native-evidence.mjs';
import { requireConsumerReports } from './consumer-reports.mjs';
import { persistenceError, reportEvidenceFailure } from './failure-diagnostics.mjs';

const canonical = path => typeof path === 'string' && isAbsolute(path) && resolve(path) === path && path !== '/';
const within = (parent, child) => {
  const path = relative(parent, child);
  return path === '' || (!isAbsolute(path) && path !== '..' && !path.startsWith(`..${sep}`));
};

/** Internal producer seam; only a verified extraction callback establishes source trust. */
export async function runConsumerGates(lease, options, runner = runChild) {
  if (!lease || typeof lease.fence !== 'function' || typeof lease.root !== 'string' || !options ||
      !isDeepStrictEqual(Object.keys(options).sort(), ['assets', 'context', 'native', 'results', 'root', 'stage']) ||
      !options.native || !isDeepStrictEqual(Object.keys(options.native).sort(), ['directory', 'record']))
    throw Error('invalid consumer execution input');
  const { root, stage, assets, results } = options;
  if (![root, stage].every(path => canonical(path) && realpathSync(path) === path) ||
      within(lease.root, stage) || within(root, stage)) throw Error('invalid consumer execution directories');
  const logs = join(stage, 'evidence');
  if (realpathSync(logs) !== logs || !lstatSync(logs).isDirectory()) throw Error('invalid consumer log directory');
  const context = structuredClone(requireConsumerContext(options.context, root));
  if (realpathSync(dirname(context.evidence_root)) !== dirname(context.evidence_root) ||
      within(root, context.evidence_root)) throw Error('invalid consumer output directory');
  const witness = structuredClone(lease.witness);
  if (context.extracted_root !== lease.root ||
      ['revision', 'tree', 'source_inventory_sha256', 'archive_sha256', 'studio_sha256'].some(key => context[key] !== witness[key]))
    throw Error('consumer source witness mismatch');
  requireCompleteGate(results, STUDIO_PROFILE, assets);
  if (results[0].cwd !== root) throw Error('consumer producer root mismatch');
  const directory = options.native.directory;
  if (typeof directory !== 'string' || !isAbsolute(directory) || resolve(directory) !== directory || realpathSync(directory) !== directory)
    throw Error('invalid consumer native directory');
  const record = structuredClone(options.native.record);
  function fence() {
    lease.fence();
    const stat = lstatSync(context.host_binary);
    if (!stat.isFile() || stat.nlink !== 1 || stat.size === 0 || !(stat.mode & 0o111) ||
        realpathSync(context.host_binary) !== context.host_binary ||
        context.host_binary !== join(directory, 'target/debug/rom-recovery-host'))
      throw Error('invalid consumer host executable');
    const provenance = safePath(directory, 'target/debug/source.json'), evidence = lstatSync(provenance);
    if (!evidence.isFile() || evidence.nlink !== 1 || evidence.size > 1024 * 1024 ||
        !isDeepStrictEqual(JSON.parse(readFileSync(provenance, 'utf8')), record))
      throw Error('consumer native provenance mismatch');
    requireNativePreparation(record, { witness, source_root: lease.root,
      target_directory: join(directory, 'target'), binary_sha256: hash(context.host_binary) }, directory);
  }
  fence();
  // Exclusive consumer output prevents reuse of earlier successful reports.
  mkdirSync(context.evidence_root);
  try {
    const bounded = (program, args, commandOptions) => runner(program, args, {
      ...commandOptions, timeout: CONSUMER_TIMEOUT_MS, maxBytes: COMMAND_LOG_BYTES,
    });
    await verifyCommands(root, stage, results, bounded, assets, {
      profile: PUBLIC_CONSUMERS_PROFILE, phase: 'consumers', context, fence,
    });
    fence();
    requireCompleteGate(results, PUBLIC_CONSUMERS_PROFILE, assets, context);
    requireConsumerReports(lease, context.evidence_root, {
      binary: context.host_binary, binary_sha256: hash(context.host_binary), lock_sha256: witness.lock_sha256,
      provenance_sha256: hash(safePath(directory, 'target/debug/source.json')),
    });
    fence();
  } catch (error) {
    const failure = { type: 'consumer-execution-failure', complete: false,
      source_witness: witness, last_recorded_gate: results.length, error: persistenceError(error) };
    try { writeFileSync(join(stage, 'evidence/consumer-failure.json'), JSON.stringify(failure, null, 2) + '\n', { flag: 'wx' }); }
    catch (storage) { reportEvidenceFailure('consumer-failure-evidence-unavailable', { failure, persistence_error: persistenceError(storage) }); }
    throw error;
  }
}
