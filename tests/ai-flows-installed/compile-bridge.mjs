// Preserve the existing checker's root-relative manifest and Cargo configuration discovery.
import { cpSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { inventory } from '../../scripts/packages/astral-adoption.mjs';

export function prepareCompileBridge(source, app) {
  mkdirSync(app); mkdirSync(join(app, 'tests'));
  const fixture = join(app, 'tests/ai-tools-compile');
  cpSync(source, fixture, { recursive: true, errorOnExist: true, force: false });
  cpSync(join(source, '.cargo'), join(app, '.cargo'), { recursive: true, errorOnExist: true, force: false });
  return { app, fixture, checker: join(fixture, 'check.mjs'), inputs: inventory(app) };
}
