// Admission primitives shared by the public source consumer and its negative cases.
import { mkdirSync, mkdtempSync, lstatSync, readFileSync, readdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { createHash } from 'node:crypto';

export function reserveOutput(path) {
  if (!path) return mkdtempSync('/var/tmp/rom-public-controls-');
  const output = resolve(path);
  try { mkdirSync(output); }
  catch (error) {
    if (error.code === 'EEXIST') throw Error('evidence output already exists');
    throw error;
  }
  return output;
}

export function assertLockedGraph(expected, actual) {
  if (!isDeepStrictEqual(expected, actual)) throw Error('dependency graph drift from frozen consumer lock');
}

export function hashBytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

export function sourceManifest(root) {
  const files = {};
  function visit(path) {
    const info = lstatSync(join(root, path));
    if (info.isSymbolicLink()) throw Error('source package contains a symbolic link');
    if (info.isDirectory()) {
      for (const entry of readdirSync(join(root, path)).sort()) visit(`${path}/${entry}`);
    } else if (info.isFile()) files[path] = hashBytes(readFileSync(join(root, path)));
    else throw Error('source package contains a non-regular file');
  }
  visit('package.json'); visit('src'); visit('LICENSE'); visit('THIRD_PARTY_NOTICES.md');
  return files;
}

export function browserOutcomes(report) {
  const outcomes = new Map();
  function visit(suite) {
    for (const spec of suite.specs ?? []) {
      for (const test of spec.tests ?? []) {
        const results = test.results ?? [];
        const statuses = outcomes.get(test.projectName) ?? [];
        statuses.push(results.length ? results.at(-1).status : 'missing');
        outcomes.set(test.projectName, statuses);
      }
    }
    for (const nested of suite.suites ?? []) visit(nested);
  }
  for (const suite of report?.suites ?? []) visit(suite);
  return Object.fromEntries(outcomes);
}

export function requireAdmission(result, browserReport) {
  if (!result.completed || result.dependency_bootstrap !== 'locked_npm_ci') throw Error('release admission requires a completed locked npm ci installation');
  if (!['release_artifact', 'extracted_release_source'].includes(result.source_mode)) throw Error('release admission requires supplied artifact source');
  if (!result.installed_source_matches) throw Error('release admission requires matching installed source');
  if (!result.browser_executed) throw Error('release admission requires browser execution');
  const outcomes = browserOutcomes(browserReport);
  for (const engine of ['chromium', 'webkit']) {
    if (!outcomes[engine]?.length) throw Error(`release admission requires executed ${engine} tests`);
    if (outcomes[engine].some(status => status !== 'passed')) throw Error(`release admission has failed or incomplete ${engine} tests`);
  }
}
