// Static preparation validation is separate from compiler and consumer execution.
import { isDeepStrictEqual } from 'node:util';
import { lstatSync, readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { safePath, hash } from '../skills/files.mjs';
import { CONSUMER_TIMEOUT_MS, COMMAND_LOG_BYTES } from './command-limits.mjs';

import { SQLITE_PROFILE, requireSqliteEngineExecution } from './sqlite-profile.mjs';
const recordKeys = ['schema_version', 'head', 'verified_source', 'source_root', 'files', 'compile_exit_code', 'binary_sha256', 'source_unchanged', 'target_directory', 'build_jobs', 'incremental', 'commands', 'log_sha256'];
const commandKeys = ['program', 'args', 'cwd', 'started_at', 'finished_at', 'exit_code', 'bounded_abort', 'spawn_failed', 'stdout', 'stderr'];
const witnessKeys = ['revision', 'tree', 'source_inventory_sha256', 'source_identity', 'archive_sha256', 'studio_sha256', 'lock_sha256'];
const exactKeys = (value, keys) => value && typeof value === 'object' && !Array.isArray(value) && isDeepStrictEqual(Object.keys(value).sort(), [...keys].sort());
const canonical = value => typeof value === 'string' && isAbsolute(value) && resolve(value) === value && value !== '/';
const sha256 = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const timestamp = value => typeof value === 'string' && /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/.test(value) && Number.isFinite(Date.parse(value)) && new Date(value).toISOString() === value;

/** Checks retained records and bytes; does not compile, run a host, or establish source trust itself. */
export function requireNativePreparation(record, expected, directory) {
  const fail = () => { throw Error('invalid native preparation evidence'); };
  try {
    if (!exactKeys(expected, ['witness', 'source_root', 'target_directory', 'binary_sha256']) ||
        !exactKeys(expected.witness, witnessKeys) ||
        !canonical(expected.source_root) || !canonical(expected.target_directory) || !sha256(expected.binary_sha256) ||
        !exactKeys(record, record.schema_version === 2 ? [...recordKeys, 'engine_profile', 'engine_verified', 'compile_profile'] : recordKeys) || ![1, 2].includes(record.schema_version) ||
        !isDeepStrictEqual(record.verified_source, expected.witness) || record.head !== expected.witness.revision ||
        record.source_root !== expected.source_root || record.target_directory !== expected.target_directory ||
        record.binary_sha256 !== expected.binary_sha256 || !exactKeys(record.files, ['Cargo.lock']) ||
        record.files['Cargo.lock'] !== expected.witness.lock_sha256 || record.compile_exit_code !== 0 ||
        record.source_unchanged !== true || record.build_jobs !== 2 || record.incremental !== false ||
        !Array.isArray(record.commands) || record.commands.length !== (record.schema_version === 2 ? 4 : 3)) fail();
    if (!/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(expected.witness.revision) ||
        !new RegExp(`^[a-f0-9]{${expected.witness.revision.length}}$`).test(expected.witness.tree) ||
        !witnessKeys.filter(key => key !== 'revision' && key !== 'tree').every(key => sha256(expected.witness[key]))) fail();
    const selected = [
      ['rustc', ['--version'], '00-rustc'],
      ['cargo', ['--version'], '01-cargo'],
      ...(record.schema_version === 2 ? [['cargo', ['test', '--locked', '--offline', '-p', 'rom-storage-conformance', '--test', 'native_profile', 'sqlite_native_engine_matches_selected_build_profile', '--', '--nocapture'], '02-engine']] : []),
      ['cargo', ['build', '--locked', '-p', 'rom-recovery-host'], '02-build'],
    ];
    if (record.schema_version === 2 && (record.engine_verified !== true || !isDeepStrictEqual(record.compile_profile, {dev_debug:false,test_debug:false}) || !isDeepStrictEqual(record.engine_profile?.profile, SQLITE_PROFILE) || !sha256(record.engine_profile.library_sha256))) fail();
    const paths = selected.flatMap(([, , stem]) => [`evidence/${stem}.stdout.log`, `evidence/${stem}.stderr.log`]);
    if (!exactKeys(record.log_sha256, paths)) fail();
    let finished = -Infinity;
    for (const [index, [program, args, stem]] of selected.entries()) {
      const command = record.commands[index];
      if (!exactKeys(command, commandKeys) || command.program !== program || !isDeepStrictEqual(command.args, args) ||
          command.cwd !== expected.source_root || command.exit_code !== 0 || command.bounded_abort !== false || command.spawn_failed !== false ||
          !timestamp(command.started_at) || !timestamp(command.finished_at) || Date.parse(command.started_at) < finished ||
          Date.parse(command.finished_at) < Date.parse(command.started_at) ||
          Date.parse(command.finished_at) - Date.parse(record.commands[0].started_at) > CONSUMER_TIMEOUT_MS ||
          command.stdout !== `evidence/${stem}.stdout.log` || command.stderr !== `evidence/${stem}.stderr.log`) fail();
      finished = Date.parse(command.finished_at);
      let bytes = 0;
      for (const path of [command.stdout, command.stderr]) {
        const full = safePath(directory, path), stat = lstatSync(full);
        if (!stat.isFile() || !sha256(record.log_sha256[path])) fail();
        bytes += stat.size;
        if (bytes > COMMAND_LOG_BYTES || hash(full) !== record.log_sha256[path]) fail();
      }
      if (index < 2) {
        const text = readFileSync(safePath(directory, command.stdout), 'utf8');
        if (text.length > 65536 || !new RegExp(`^${program} \\d+\\.\\d+\\.\\d+[^\\r\\n]*\\n?$`).test(text)) fail();
      }
    }
    if (record.schema_version === 2) requireSqliteEngineExecution(readFileSync(safePath(directory, 'evidence/02-engine.stdout.log'), 'utf8'));
    return record;
  } catch (error) {
    if (error.message === 'invalid native preparation evidence') throw error;
    throw Error('invalid native preparation evidence', { cause: error });
  }
}
