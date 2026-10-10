// Validate retained public engine build records; Rust linkage needs its separate executed test.
import { readFileSync, lstatSync } from 'node:fs';
import { join, isAbsolute, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { createHash } from 'node:crypto';
import { hash, safePath } from '../skills/files.mjs';
import { SQLITE_PROFILE, requireSqliteHeader } from './sqlite-profile.mjs';

const sha3 = path => createHash('sha3-256').update(readFileSync(path)).digest('hex');
const digest = value => /^[a-f0-9]{64}$/.test(value ?? '');
const canonical = value => typeof value === 'string' && isAbsolute(value) && resolve(value) === value && value !== '/';
const timestamp = value => typeof value === 'string' && Number.isFinite(Date.parse(value)) && new Date(value).toISOString() === value;

export function requireSqliteBuildCommands(record, directory) {
  if (!Array.isArray(record.commands) || record.commands.length !== 2 ||
      Object.keys(record.tools ?? {}).sort().join(',') !== 'ar,cc') throw Error('invalid SQLite build evidence');
  const root = record.commands[0].cwd;
  if (!canonical(root)) throw Error('invalid SQLite build evidence');
  const source = join(root, 'source/sqlite-autoconf-3530400/sqlite3.c');
  const selected = [
    ['cc', ['-O2', '-fPIC', '-DSQLITE_THREADSAFE=1', '-DSQLITE_ENABLE_COLUMN_METADATA', '-c', source, '-o', join(root, 'sqlite3.o')], 'sqlite-compile'],
    ['ar', ['rcs', join(root, 'lib/libsqlite3.a'), join(root, 'sqlite3.o')], 'sqlite-archive'],
  ];
  const paths = selected.flatMap(([, , stem]) => [`evidence/${stem}.stdout.log`, `evidence/${stem}.stderr.log`]);
  if (!isDeepStrictEqual(Object.keys(record.log_sha256 ?? {}).sort(), paths.sort())) throw Error('invalid SQLite build evidence');
  let finished = -Infinity;
  for (const [index, [name, args, stem]] of selected.entries()) {
    const tool = record.tools[name], command = record.commands[index];
    if (!canonical(tool.path) || !digest(tool.sha256) || command.program !== tool.path ||
        !isDeepStrictEqual(command.args, args) || command.cwd !== root || command.exit_code !== 0 ||
        command.bounded_abort !== false || command.spawn_failed !== false ||
        !timestamp(command.started_at) || !timestamp(command.finished_at) ||
        Date.parse(command.started_at) < finished || Date.parse(command.finished_at) < Date.parse(command.started_at) ||
        Date.parse(command.finished_at) - Date.parse(command.started_at) > 240000 ||
        Date.parse(command.finished_at) - Date.parse(record.commands[0].started_at) > 480000 ||
        command.stdout !== `evidence/${stem}.stdout.log` || command.stderr !== `evidence/${stem}.stderr.log`)
      throw Error('invalid SQLite build evidence');
    finished = Date.parse(command.finished_at);
    let bytes = 0;
    for (const path of [command.stdout, command.stderr]) {
      const full = safePath(directory, path), stat = lstatSync(full);
      bytes += stat.size;
      if (!stat.isFile() || stat.nlink !== 1 || bytes > 32 * 1024 * 1024 ||
          !digest(record.log_sha256[path]) || hash(full) !== record.log_sha256[path]) throw Error('invalid SQLite build evidence');
    }
  }
  return true;
}

export function requireSqliteBuildEvidence(record, directory) {
  if (record.schema_version !== 1 || !isDeepStrictEqual(record.profile, SQLITE_PROFILE) ||
      record.linked_engine_verified !== false ||
      sha3(safePath(directory, 'sqlite-autoconf-3530400.tar.gz')) !== SQLITE_PROFILE.archive_sha3_256 ||
      sha3(safePath(directory, 'source/sqlite-autoconf-3530400/sqlite3.c')) !== SQLITE_PROFILE.sqlite3_c_sha3_256)
    throw Error('invalid SQLite source evidence');
  for (const [path, key] of [['sqlite-autoconf-3530400.tar.gz', 'archive_sha256'],
    ['source/sqlite-autoconf-3530400/sqlite3.c', 'sqlite3_c_sha256'], ['include/sqlite3.h', 'sqlite3_h_sha256'],
    ['lib/libsqlite3.a', 'library_sha256'], ['lib/pkgconfig/sqlite3.pc', 'pkg_config_sha256']]) {
    const full = safePath(directory, path), stat = lstatSync(full);
    if (!stat.isFile() || stat.nlink !== 1 || stat.size < 1 || stat.size > 64 * 1024 * 1024 ||
        !digest(record[key]) || hash(full) !== record[key]) throw Error('invalid SQLite retained bytes');
  }
  requireSqliteHeader(readFileSync(safePath(directory, 'include/sqlite3.h'), 'utf8'));
  requireSqliteBuildCommands(record, directory);
  return true;
}
