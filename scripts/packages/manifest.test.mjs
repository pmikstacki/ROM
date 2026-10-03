import assert from 'node:assert/strict';
import { test } from 'node:test';
import { renderManifest } from './manifest.mjs';
const dep = (name, kind = null, other = {}) => ({name, kind, req: '=1.0.0', uses_default_features: true, features: [], optional: false, target: null, ...other});
const pkg = { name: 'reference', version: '0.1.0-alpha.1', edition: '2024', rust_version: '1.99', features: { default: [], provider: ['dep:auth-client'] }, dependencies: [dep('core'), dep('auth', null, {rename: 'auth-client', optional: true, uses_default_features: false, features: ['introspection']}), dep('test-fixture', 'dev'), dep('builder', 'build', {target: 'cfg(unix)'})] };
test('external application manifest retains exact identity, optional features and dependency scopes', () => {
  const text = renderManifest(pkg);
  assert.match(text, /name = "reference"/);
  assert.match(text, /version = "0.1.0-alpha.1"/);
  assert.match(text, /rust-version = "1.99"/);
  assert.match(text, /publish = false/);
  assert.match(text, /"provider" = \["dep:auth-client"\]/);
  assert.match(text, /"auth-client" = \{ version = "=1.0.0", default-features = false, features = \["introspection"\], optional = true, package = "auth" \}/);
  assert.match(text, /\[dev-dependencies\]\n"test-fixture"/);
  assert.match(text, /\[target\."cfg\(unix\)"\.build-dependencies\]\n"builder"/);
  assert.match(text, /\[workspace\]/);
  assert.doesNotMatch(text, /path =/);
});
test('unsupported dependency kinds and alternate registries fail explicitly', () => {
  assert.throws(() => renderManifest({...pkg,dependencies:[dep('bad','unrecognized')]}), /dependency kind/);
  assert.throws(() => renderManifest({...pkg,dependencies:[dep('custom',null,{registry:'https://private.invalid/index'})]}), /registry/);
});
