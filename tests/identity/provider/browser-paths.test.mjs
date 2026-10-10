import test from 'node:test';
import assert from 'node:assert/strict';
import { requireBrowserSocketBudget } from './browser-paths.mjs';

test('retained long fixture TMPDIR fails before Chromium launch with a closed diagnostic', () => {
  const retained = '/var/tmp/rom-010-authentik-20261007/run/volume/private/host-authoring-6f220c250fbfb681942d8fb86d667fbf/sqlite/tmp';
  assert.throws(() => requireBrowserSocketBudget(retained), /browser TMPDIR exceeds Unix socket path budget; use a short owned directory inside the run filesystem/);
});
test('short owned TMPDIR retains space for the Chromium socket suffix', () => {
  assert.doesNotThrow(() => requireBrowserSocketBudget('/var/tmp/rom-010-authentik-20261007/run/volume/t-12345678'));
  assert.doesNotThrow(() => requireBrowserSocketBudget('/' + 'a'.repeat(58)));
  assert.throws(() => requireBrowserSocketBudget('/' + 'a'.repeat(59)));
});
test('socket budget counts UTF-8 bytes and rejects non-absolute or malformed paths', () => {
  assert.doesNotThrow(() => requireBrowserSocketBudget('/' + 'é'.repeat(29)));
  assert.throws(() => requireBrowserSocketBudget('/' + 'é'.repeat(30)));
  for (const value of ['', 'relative', '/tmp/../other', '/tmp\0other', null]) assert.throws(() => requireBrowserSocketBudget(value));
});
