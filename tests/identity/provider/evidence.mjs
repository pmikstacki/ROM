// Closed public result schema. Tokens, cookies and arbitrary diagnostics have no field.
import { writeFileSync, lstatSync, realpathSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { SHA256, exactKeys } from './admission.mjs';

const id = value => typeof value === 'string' && /^[a-z][a-z0-9-]{0,63}$/.test(value);
const integer = value => Number.isSafeInteger(value) && value >= 0;

export function requireAdmission(report, requiredCases) {
  const fail = () => { throw Error('invalid identity provider admission'); };
  if (!exactKeys(report, ['schema', 'source_mode', 'source_sha256', 'final_source_sha256', 'dependency_mode', 'provider_admitted', 'tls_verified', 'processes', 'cases']) ||
      report.schema !== 'rom-identity-provider-v1' || report.source_mode !== 'verified_extracted_source' ||
      typeof report.source_sha256 !== 'string' || !SHA256.test(report.source_sha256) || report.source_sha256 !== report.final_source_sha256 ||
      report.dependency_mode !== 'locked_npm_ci' || report.provider_admitted !== true || report.tls_verified !== true ||
      !Array.isArray(requiredCases) || !requiredCases.length || requiredCases.length > 64 || !requiredCases.every(id) || new Set(requiredCases).size !== requiredCases.length ||
      !Array.isArray(report.processes) || !report.processes.length || report.processes.length > 32 || !Array.isArray(report.cases)) fail();
  const pids = new Set();
  for (const process of report.processes) {
    if (!exactKeys(process, ['pid', 'exit_code', 'signal', 'reason', 'drained'])) fail();
    const normal = process.exit_code === 0 && process.signal === null;
    const graceful = process.reason === 'shutdown' && (normal || process.exit_code === null && process.signal === 'SIGTERM');
    if (!Number.isSafeInteger(process.pid) || process.pid <= 1 ||
        pids.has(process.pid) || !(process.reason === 'completed' && normal || graceful) || process.drained !== true) fail();
    pids.add(process.pid);
  }
  const expected = new Set(['sqlite', 'redb'].flatMap(adapter => ['chromium', 'webkit'].flatMap(engine => requiredCases.map(name => `${adapter}/${engine}/${name}`))));
  if (report.cases.length !== expected.size) fail();
  for (const entry of report.cases) {
    if (!exactKeys(entry, ['id', 'adapter', 'engine', 'status', 'elapsed_ms']) || !id(entry.id) ||
        !['sqlite', 'redb'].includes(entry.adapter) || !['chromium', 'webkit'].includes(entry.engine) ||
        entry.status !== 'passed' || !integer(entry.elapsed_ms) || entry.elapsed_ms > 180_000 ||
        !expected.delete(`${entry.adapter}/${entry.engine}/${entry.id}`)) fail();
  }
  if (expected.size) fail();
  return report;
}

export function writeEvidence(root, name, value, limit = 1024 * 1024) {
  if (typeof root !== 'string' || resolve(root) !== root || realpathSync(root) !== root || !lstatSync(root).isDirectory()) throw Error('invalid evidence root');
  if (typeof name !== 'string' || !/^[a-z][a-z0-9-]{0,63}\.json$/.test(name)) throw Error('invalid evidence name');
  if (!Number.isSafeInteger(limit) || limit <= 0 || limit > 32 * 1024 * 1024) throw Error('invalid evidence limit');
  function inspect(item, depth = 0) {
    if (depth > 32) throw Error('private evidence depth limit');
    if (typeof item === 'string' && /(?:bearer\s|authorization:|(?:password|cookie|token|secret)=)/i.test(item)) throw Error('private evidence rejected');
    if (item && typeof item === 'object') {
      for (const [key, nested] of Object.entries(item)) {
        if (/(?:^|_)(?:password|secret|cookie|token|credentials|authorization|stderr|stdout)(?:$|_)/i.test(key)) throw Error('private evidence rejected');
        inspect(nested, depth + 1);
      }
    }
  }
  inspect(value);
  const bytes = Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
  if (bytes.length > limit) throw Error('evidence limit exceeded');
  try { writeFileSync(join(root, name), bytes, { flag: 'wx', mode: 0o600 }); }
  catch (error) { if (error.code === 'EEXIST') throw Error('evidence already exists'); throw error; }
}
