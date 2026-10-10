// Strict scenario/result admission does not prove runtime provenance or replace source checks.
const engines = Object.freeze(['chromium', 'webkit']);
export const controlsScenarios = Object.freeze([
  ['installed public controls preserve bound values, native refs and keyboard operations', 'public-controls.spec.ts'],
  ['installed stylesheet and wrapper contracts support a compact composer at 1280x900', 'public-controls.spec.ts'],
  ['installed stylesheet and wrapper contracts support a compact composer at 390x500', 'public-controls.spec.ts'],
  ['installed public client and recovery entries preserve exact typed mutation draft', 'public-controls.spec.ts'],
  ['installed root App and renderer compose resources settings and work at 1440px', 'public-root.spec.ts'],
  ['installed root App and renderer compose resources settings and work at 390px', 'public-root.spec.ts'],
].map(([title, file]) => Object.freeze({ title, file })));
export const recoveryScenarios = Object.freeze([
  'lost acknowledgement, restart, IndexedDB reload and renewed session preserve A while retaining C',
  'current revocation and principal switch hide results and never retry another owner automatically',
  'invalid editor text survives reload and blocked navigation separately from the accepted command',
  'unrelated inventory editor composes the helper with its own validation and navigation policy',
  'principal change while a committed real HTTP result is withheld rejects late disclosure',
].map(title => Object.freeze({ title, file: 'recovery.spec.ts' })));

const fail = () => { throw Error('invalid consumer browser evidence'); };
function requireResult(spec, test) {
  if (spec.ok !== true || test.expectedStatus !== 'passed' || test.status !== 'expected' ||
      !Array.isArray(test.results) || test.results.length !== 1) fail();
  const result = test.results[0];
  if (!result || result.status !== 'passed' || result.retry !== 0 || result.error ||
      !Array.isArray(result.errors) || result.errors.length !== 0) fail();
}

export function requireControlsBrowserReport(report) {
  return requireBrowserReport(report, controlsScenarios);
}
export function requireRecoveryBrowserReport(report) {
  return requireBrowserReport(report, recoveryScenarios);
}
function requireBrowserReport(report, requiredScenarios) {
  const projects = report?.config?.projects;
  if (!Array.isArray(projects) || projects.length !== engines.length ||
      !projects.every(project => project && engines.includes(project.name)) ||
      JSON.stringify(projects.map(project => project.name).sort()) !== JSON.stringify(engines) ||
      !Array.isArray(report.errors) || report.errors.length !== 0 || !Array.isArray(report.suites)) fail();
  const expectedCount = engines.length * requiredScenarios.length;
  if (report.stats?.expected !== expectedCount || ['skipped', 'unexpected', 'flaky'].some(key => report.stats[key] !== 0)) fail();
  const scenarios = new Map(requiredScenarios.map(scenario => [scenario.title, scenario.file]));
  const executed = new Set(), pending = report.suites.map(suite => ({ suite, depth: 0 }));
  let visited = 0;
  while (pending.length) {
    const { suite, depth } = pending.pop();
    if (!suite || ++visited > 10_000 || depth > 32 ||
        (suite.specs !== undefined && !Array.isArray(suite.specs)) ||
        (suite.suites !== undefined && !Array.isArray(suite.suites)) ||
        (suite.suites?.length ?? 0) + pending.length > 10_000) fail();
    for (const spec of suite.specs ?? []) {
      if (!spec || scenarios.get(spec.title) !== spec.file || !Array.isArray(spec.tests) || !spec.tests.length) fail();
      for (const test of spec.tests) {
        if (!test || !engines.includes(test.projectName)) fail();
        requireResult(spec, test);
        const identity = `${test.projectName}:${spec.title}`;
        if (executed.has(identity)) fail();
        executed.add(identity);
      }
    }
    for (const child of suite.suites ?? []) pending.push({ suite: child, depth: depth + 1 });
  }
  if (executed.size !== expectedCount) fail();
  return { engines: [...engines], executions: executed.size, scenarios: requiredScenarios.map(scenario => ({ ...scenario })) };
}
