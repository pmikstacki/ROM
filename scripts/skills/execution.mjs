// Bounded child ownership and retained local example evidence.
import { mkdtempSync, writeFileSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { safePath } from './files.mjs';
import { rustProject } from './rust-project.mjs';
import { runChild } from './process.mjs';

export async function execute(context) {
  const evidence = mkdtempSync(join(tmpdir(), `rom-skill-${context.workflow}-`));
  const commands = [];
  async function command(program, args, options = {}) {
    const record = { program, args, cwd: options.cwd ?? context.root };
    const index = commands.push(record);
    const result = await runChild(program, args, { cwd: record.cwd, env: { ...process.env, CARGO_BUILD_JOBS: '2', CARGO_PROFILE_DEV_DEBUG: '0', CARGO_PROFILE_TEST_DEBUG: '0' }, timeout: options.timeout });
    writeFileSync(join(evidence, `command-${index}.log`), result.stdout + result.stderr);
    record.code = result.code;
    record.timed_out = result.timedOut;
    if (result.timedOut || (result.code !== (options.expected ?? 0))) throw Error(`example command ${index} failed; evidence ${evidence}`);
    return result;
  }
  try {
    const asset = await import(pathToFileURL(safePath(context.bundle, context.selected.asset)));
    const runtime = { ...context, evidence, command, project: join(evidence, 'project') };
    runtime.rust = name => rustProject(runtime, name);
    const result = await asset.run(runtime);
    writeFileSync(join(evidence, 'result.json'), JSON.stringify({ workflow: context.workflow, source: context.manifest.source, commands, result, status: 'passed' }, null, 2) + '\n');
    console.log(`Example passed. Evidence: ${evidence}`);
  } catch (error) {
    mkdirSync(evidence, { recursive: true });
    writeFileSync(join(evidence, 'result.json'), JSON.stringify({ workflow: context.workflow, commands, status: 'failed' }, null, 2) + '\n');
    throw error;
  }
}
