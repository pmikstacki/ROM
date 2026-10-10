// Report checks supplement subprocess status; they do not establish source trust themselves.
import { readFileSync, lstatSync, realpathSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { safePath, hash } from '../skills/files.mjs';
import { requireConsumerFiles } from './consumer-files.mjs';
import { requireRecoveryFixture } from './consumer-fixture-evidence.mjs';
import { requireRecoveryHttpReport } from './consumer-http-evidence.mjs';
import { requireControlsBrowserReport, requireRecoveryBrowserReport } from './consumer-evidence.mjs';

function readReport(directory, path) {
  const full = safePath(directory, path), stat = lstatSync(full);
  if (!stat.isFile() || stat.nlink !== 1 || stat.size > 8 * 1024 * 1024) throw Error('invalid report file');
  return JSON.parse(readFileSync(full, 'utf8'));
}

/** Call only with the still-live extraction lease used to execute the three fixed consumers. */
export function requireConsumerReports(lease, directory, native, context) {
  try {
    if (!native || !isDeepStrictEqual(Object.keys(native).sort(), ['binary', 'binary_sha256', 'lock_sha256', 'provenance_sha256']) ||
        typeof native.binary !== 'string' || !isAbsolute(native.binary) || resolve(native.binary) !== native.binary ||
        !['binary_sha256', 'lock_sha256', 'provenance_sha256'].every(key => /^[a-f0-9]{64}$/.test(native[key] ?? '')))
      throw Error('invalid native identity expectations');
    if (realpathSync(directory) !== directory || !lstatSync(directory).isDirectory()) throw Error('invalid report directory');
    const reports = [];
    for (const kind of ['controls', 'recovery-sqlite', 'recovery-redb']) {
      const child = safePath(directory, kind);
      const result = readReport(child, 'result.json');
      requireConsumerFiles(lease, child, kind, result, context);
      if (kind !== 'controls') {
        requireRecoveryFixture(lease, child, result);
        const http = readReport(child, 'http-results.json');
        if (!isDeepStrictEqual(http, result.http)) throw Error('retained HTTP evidence differs');
        requireRecoveryHttpReport(http, kind.slice(9));
        if (result.native_binary !== native.binary || result.native_binary_sha256 !== native.binary_sha256 ||
            result.native_compile_lock_sha256 !== native.lock_sha256 || result.current_root_lock_sha256 !== native.lock_sha256 ||
            result.native_provenance_sha256 !== native.provenance_sha256) throw Error('consumer native identity differs');
        readReport(child, 'native-provenance.json');
        if (hash(safePath(child, 'native-provenance.json')) !== native.provenance_sha256) throw Error('copied native provenance differs');
      }
      const browser = readReport(child, 'browser-results.json');
      const verified = kind === 'controls' ? requireControlsBrowserReport(browser) : requireRecoveryBrowserReport(browser);
      reports.push({ kind, ...verified });
    }
    lease.fence();
    return reports;
  } catch (error) {
    throw Error('invalid consumer evidence', { cause: error });
  }
}
