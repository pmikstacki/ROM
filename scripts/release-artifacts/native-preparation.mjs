// Native fixture preparation runs only inside an existing verified source lease.
import { mkdirSync, writeFileSync, readFileSync, lstatSync, realpathSync, copyFileSync, renameSync } from 'node:fs';
import { join, resolve, relative, isAbsolute, dirname, sep } from 'node:path';
import { createHash } from 'node:crypto';
import { recordedCommand } from './command-result.mjs';
import { runChild } from '../skills/process.mjs';
import { persistenceError, reportEvidenceFailure } from './failure-diagnostics.mjs';
import { sqliteProfileEnvironment, requireSqliteEngineExecution } from './sqlite-profile.mjs';
import { CONSUMER_TIMEOUT_MS, COMMAND_LOG_BYTES } from './command-limits.mjs';

const digest = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const within = (parent, child) => {
  const path = relative(parent, child);
  return path === '' || (!isAbsolute(path) && path !== '..' && !path.startsWith(`..${sep}`));
};

/** Internal producer helper. A caller-supplied object alone is not verified-source admission. */
export async function prepareConsumerHost(lease, directory, runner = runChild, profile) {
  if (!lease || typeof lease.fence !== 'function' || !isAbsolute(lease.root ?? '') ||
      resolve(lease.root) !== lease.root || !isAbsolute(directory) || resolve(directory) !== directory ||
      within(lease.root, directory) || within(directory, lease.root) ||
      realpathSync(lease.root) !== lease.root || realpathSync(dirname(directory)) !== dirname(directory) ||
      !lease.witness || !['lock_sha256', 'source_inventory_sha256', 'archive_sha256', 'studio_sha256', 'source_identity'].every(key => /^[a-f0-9]{64}$/.test(lease.witness[key] ?? '')) ||
      !/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(lease.witness.revision ?? '') ||
      !new RegExp(`^[a-f0-9]{${lease.witness.revision.length}}$`).test(lease.witness.tree ?? ''))
    throw Error('invalid native preparation source or output');
  lease.fence();
  if (digest(join(lease.root, 'Cargo.lock')) !== lease.witness.lock_sha256)
    throw Error('native preparation lock mismatch');
  mkdirSync(directory);
  mkdirSync(join(directory, 'evidence'));
  const target = join(directory, 'target');
  const commands = [];
  const deadline = performance.now() + CONSUMER_TIMEOUT_MS;
  const bounded = (program, args, options) => {
    const remaining = Math.floor(deadline - performance.now());
    if (remaining <= 0) return { code: null, timedOut: true, stdout: '', stderr: 'native preparation deadline exceeded' };
    return runner(program, args, {
    ...options,
    env: { ...(profile ? sqliteProfileEnvironment(options.env, profile.directory) : options.env), CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0', CARGO_TARGET_DIR: target, ...(profile ? { CARGO_PROFILE_DEV_DEBUG: '0', CARGO_PROFILE_TEST_DEBUG: '0' } : {}) },
    timeout: remaining,
    maxBytes: COMMAND_LOG_BYTES,
    });
  };
  try {
    const selected = [
      ['rustc', ['--version'], '00-rustc'],
      ['cargo', ['--version'], '01-cargo'],
      ...(profile ? [['cargo', ['test', '--locked', '--offline', '-p', 'rom-storage-conformance', '--test', 'native_profile', 'sqlite_native_engine_matches_selected_build_profile', '--', '--nocapture'], '02-engine']] : []),
      ['cargo', ['build', '--locked', '-p', 'rom-recovery-host'], '02-build'],
    ];
    for (const [program, args, stem] of selected) {
      const command = await recordedCommand(program, args, lease.root, directory, stem, bounded);
      commands.push(command);
      if (command.exit_code !== 0 || command.bounded_abort || command.spawn_failed)
        throw Error('native preparation command failed');
    }
    if (profile) requireSqliteEngineExecution(readFileSync(join(directory, 'evidence/02-engine.stdout.log'), 'utf8'));
    lease.fence();
    if (performance.now() > deadline) throw Error('native preparation deadline exceeded');
    if (digest(join(lease.root, 'Cargo.lock')) !== lease.witness.lock_sha256)
      throw Error('native preparation lock mismatch');
    const binary = join(target, 'debug/rom-recovery-host');
    const compiled = lstatSync(binary);
    if (!compiled.isFile() || !compiled.size || !(compiled.mode & 0o111) || realpathSync(binary) !== binary) throw Error('native preparation binary is not a private executable');
    if (profile && compiled.nlink > 1) { const detached = binary + '.detached'; copyFileSync(binary, detached); renameSync(detached, binary); }
    const stat = lstatSync(binary);
    if (!stat.isFile() || stat.nlink !== 1 || stat.size === 0 || !(stat.mode & 0o111) ||
        realpathSync(binary) !== binary) throw Error('native preparation binary is not a private executable');
    const provenance = join(dirname(binary), 'source.json');
    const record = {
      schema_version: profile ? 2 : 1, ...(profile ? { engine_profile: profile.record, engine_verified: true, compile_profile: { dev_debug: false, test_debug: false } } : {}), head: lease.witness.revision, verified_source: lease.witness,
      source_root: lease.root, files: { 'Cargo.lock': digest(join(lease.root, 'Cargo.lock')) },
      compile_exit_code: 0, binary_sha256: digest(binary), source_unchanged: true,
      target_directory: target, build_jobs: 2, incremental: false, commands,
      log_sha256: Object.fromEntries(commands.flatMap(command => [command.stdout, command.stderr]).map(path => [path, digest(join(directory, path))])),
    };
    writeFileSync(provenance, JSON.stringify(record, null, 2) + '\n', { flag: 'wx' });
    return Object.freeze({ binary, provenance, record });
  } catch (error) {
    try {
      writeFileSync(join(directory, 'failure.json'), JSON.stringify({ completed: false, error: error.message, commands }, null, 2) + '\n', { flag: 'wx' });
    } catch (storageError) {
      reportEvidenceFailure('native-preparation-evidence-failure', {
        directory, original_error: persistenceError(error), persistence_error: persistenceError(storageError),
        last_command: commands.at(-1) ?? error.commandResult ?? null,
      });
    }
    throw error;
  }
}
