import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { hash, digest } from '../skills/files.mjs';
import { inventory } from './astral-adoption.mjs';
import { requirePackageProvenance, fenceProducer } from '../../tests/ai-flows-installed/provenance.mjs';
import { prepareCompileBridge } from '../../tests/ai-flows-installed/compile-bridge.mjs';
import { rewriteAiManifest, auditAiGraph, requireAiResults, requireAiCompiler, fenceInputs, validateOptions, runAiConsumer, auditRegistryLock } from './ai-flows-consumer.mjs';

test('example manifest keeps features while removing checkout routes and rejecting unreviewed patches', () => {
  const packages = [{ name: 'rom', version: '0.1.0' }, { name: 'rom-ai', version: '0.1.0' }];
  const input = '[workspace]\n[dependencies]\nrom = { version = "=0.0.3", path = "../../crates/rom" }\nrom-ai = { version = "=0.0.3", path = "../../crates/rom-ai", features = ["test-support"] }\n';
  assert.equal(rewriteAiManifest(input, packages), '[workspace]\n[dependencies]\nrom = { version = "=0.1.0" }\nrom-ai = { version = "=0.1.0", features = ["test-support"] }\n');
  assert.throws(() => rewriteAiManifest(input + '[patch.crates-io]\n', packages), /override/);
  assert.throws(() => rewriteAiManifest(input.replace('../../crates/rom-ai', '/unreviewed/rom-ai'), packages), /path/);
});

test('resolved graph rejects even one ROM dependency outside its exact package and foreign local crates', () => {
  const packages = [{ name: 'rom', version: '0.1.0', directory: '/extracted/rom-0.1.0' }, { name: 'rom-ai', version: '0.1.0', directory: '/extracted/rom-ai-0.1.0' }];
  const graph = { workspace_root: '/isolated/app', packages: [
    { id: 'app', name: 'rom-ai-flows-consumer', version: '0.0.0', source: null, manifest_path: '/isolated/app/Cargo.toml' },
    { id: 'rom', name: 'rom', version: '0.1.0', source: null, manifest_path: '/extracted/rom-0.1.0/Cargo.toml' },
    { id: 'ai', name: 'rom-ai', version: '0.1.0', source: null, manifest_path: '/extracted/rom-ai-0.1.0/Cargo.toml' },
  ], resolve: { nodes: [{ id: 'app' }, { id: 'rom' }, { id: 'ai' }] } };
  auditAiGraph(graph, packages, '/isolated/app', 'rom-ai-flows-consumer');
  for (const replacement of [ '/checkout/crates/rom/Cargo.toml', '/extracted/rom-0.1.0/nested/Cargo.toml' ]) {
    const bad = structuredClone(graph); bad.packages[1].manifest_path = replacement;
    assert.throws(() => auditAiGraph(bad, packages, '/isolated/app', 'rom-ai-flows-consumer'), /escaped/);
  }
  const bad = structuredClone(graph); bad.packages[0].name = 'foreign-local';
  assert.throws(() => auditAiGraph(bad, packages, '/isolated/app', 'rom-ai-flows-consumer'), /local/);
});

test('recorded native oracle requires actual two-adapter domain tests and rejects partial or failed runs', () => {
  const names = [
    'cancellation::redb_pending_successor_cancellation_preserves_hold_after_reopen',
    'cancellation::sqlite_pending_successor_cancellation_preserves_hold_after_reopen',
    'redb_failover_current_grant_revoked_before_retry_never_retransmits',
    'redb_failover_exact_commits_recover_lost_acknowledgements',
    'redb_failover_exact_reference_survives_due_and_reopen',
    'redb_failover_exhaustion_is_durable_and_keeps_original_evidence',
    'redb_failover_fresh_catalog_rejects_now_overpriced_alternate',
    'redb_failover_generation_is_held_without_zero_settlement',
    'redb_failover_known_cost_counts_before_successor_reservation',
    'redb_failover_lost_waiting_ack_preserves_receipt_and_single_successor',
    'redb_failover_original_deadline_before_rounded_due_never_retransmits',
    'redb_failover_positive_reconciliation_preserves_known_charge',
    'redb_failover_rejects_forged_or_rewritten_persisted_decisions',
    'redb_failover_two_unrelated_runs_have_independent_durable_cursors',
    'redb_failover_uncertain_known_cost_stays_charged_without_retry',
    'redb_failover_untrusted_usage_retains_reserved_ceiling_after_reopen',
    'regressions::redb_overlapping_prepared_callbacks_dispatch_alternate_once',
    'regressions::redb_prepared_attempt_only_denial_is_public_terminal',
    'regressions::redb_prepared_expiry_in_preflight_fails_and_releases_after_reopen',
    'regressions::redb_terminal_tombstone_fences_late_uncommitted_reserve',
    'regressions::sqlite_overlapping_prepared_callbacks_dispatch_alternate_once',
    'regressions::sqlite_prepared_attempt_only_denial_is_public_terminal',
    'regressions::sqlite_prepared_expiry_in_preflight_fails_and_releases_after_reopen',
    'regressions::sqlite_terminal_tombstone_fences_late_uncommitted_reserve',
    'sqlite_failover_current_grant_revoked_before_retry_never_retransmits',
    'sqlite_failover_exact_commits_recover_lost_acknowledgements',
    'sqlite_failover_exact_reference_survives_due_and_reopen',
    'sqlite_failover_exhaustion_is_durable_and_keeps_original_evidence',
    'sqlite_failover_fresh_catalog_rejects_now_overpriced_alternate',
    'sqlite_failover_generation_is_held_without_zero_settlement',
    'sqlite_failover_known_cost_counts_before_successor_reservation',
    'sqlite_failover_lost_waiting_ack_preserves_receipt_and_single_successor',
    'sqlite_failover_original_deadline_before_rounded_due_never_retransmits',
    'sqlite_failover_positive_reconciliation_preserves_known_charge',
    'sqlite_failover_rejects_forged_or_rewritten_persisted_decisions',
    'sqlite_failover_two_unrelated_runs_have_independent_durable_cursors',
    'sqlite_failover_uncertain_known_cost_stays_charged_without_retry',
    'sqlite_failover_untrusted_usage_retains_reserved_ceiling_after_reopen',
    'redb_completed_tool_grant_revocation_is_terminal',
    'redb_publication_completed_tools_survive_model_failover',
    'redb_triage_completed_tools_survive_model_failover',
    'sqlite_completed_tool_grant_revocation_is_terminal',
    'sqlite_publication_completed_tools_survive_model_failover',
    'sqlite_triage_completed_tools_survive_model_failover',
    'sqlite_public_publication_flow_prepares_then_publishes_with_retained_sources', 'redb_public_publication_flow_prepares_then_publishes_with_retained_sources',
    'sqlite_public_ticket_triage_flow_reads_then_classifies_once', 'redb_public_ticket_triage_flow_reads_then_classifies_once',
    ...['sqlite', 'redb'].flatMap(adapter => [
      `${adapter}_external_consumers_recover_original_tool_receipt_after_lost_ack`,
      `${adapter}_external_consumers_recheck_current_tool_grant_before_receipt_recovery`,
      `${adapter}_external_consumers_cancel_unknown_tool_effect_without_resetting_history`,
      `${adapter}_external_consumers_report_generation_budget_after_committed_domain_stage`,
      `${adapter}_publication_retains_immutable_revision_before_head_commit`,
      `${adapter}_unrelated_triage_rejects_invalid_classification_and_replays_exact_action`,
    ]),
    'external_custom_classification_field_uses_public_codec_and_rejects_unknown_values',
  ];
  assert.equal(new Set(names).size, names.length);
  const log = names.map(name => `test ${name} ... ok`).join('\n') + `\ntest result: ok. ${names.length} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1s\n`;
  assert.equal(requireAiResults(log, 'flows').passed, 61);
  for (const name of names) {
    assert.throws(
      () => requireAiResults(log.replace(`test ${name} ... ok`, `test omitted_${name} ... ok`), 'flows'),
      error => error.message === `missing executed domain test: ${name}`,
      name,
    );
  }
  const legacyNames = names.slice(44);
  const legacyLog = legacyNames.map(name => `test ${name} ... ok`).join('\n') + '\ntest result: ok. 17 passed; 0 failed; 0 ignored;\n';
  assert.throws(() => requireAiResults(legacyLog, 'flows'), /missing executed domain test/);
  assert.throws(() => requireAiResults(log.replace('0 failed', '1 failed'), 'flows'), /failed/);
  assert.throws(() => requireAiResults(log.replace('0 ignored', '1 ignored'), 'flows'), /ignored/);
  assert.throws(() => requireAiResults('test result: ok. 0 passed; 0 failed; 0 ignored;', 'adapter'), /empty/);
});

test('compiler contract rejects a different release and records its full identity', () => {
  assert.equal(requireAiCompiler('rustc 1.99.0 (abc)\nrelease: 1.99.0\nhost: x86_64-unknown-linux-gnu\n', '1.99.0').release, '1.99.0');
  assert.throws(() => requireAiCompiler('rustc 1.100.0\nrelease: 1.100.0\nhost: x86_64-unknown-linux-gnu\n', '1.99.0'), /compiler/);
});

test('input fence detects changed lock and newly inserted source without deleting historical evidence', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-ai-fence-test-')); mkdirSync(join(root, 'app'));
  const app = join(root, 'app'); writeFileSync(join(app, 'Cargo.lock'), 'original');
  const expected = { 'Cargo.lock': hash(join(app, 'Cargo.lock')) };
  fenceInputs(app, expected); writeFileSync(join(app, 'Cargo.lock'), 'changed');
  assert.throws(() => fenceInputs(app, expected), /changed/);
  writeFileSync(join(app, 'Cargo.lock'), 'original'); writeFileSync(join(app, 'injected.rs'), 'injection');
  assert.throws(() => fenceInputs(app, expected), /changed/);
  assert.equal(readFileSync(join(app, 'Cargo.lock'), 'utf8'), 'original');
});

test('native runner requires explicit finite lease budgets before creating output', () => {
  const parent = mkdtempSync(join(tmpdir(), 'rom-ai-options-test-'));
  const output = join(parent, 'never-created');
  assert.throws(() => validateOptions({ output }), /budget/);
  assert.equal(existsSync(output), false);
});

test('runner rejects missing lease inputs before native execution or output allocation', async () => {
  await assert.rejects(runAiConsumer({}), /budget/);
});

test('lock admission rejects unexplained registry version and checksum drift', () => {
  const lock = '[[package]]\nname = "tokio"\nversion = "1.53.1"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "' + 'a'.repeat(64) + '"\n';
  auditRegistryLock(lock, [lock]);
  assert.throws(() => auditRegistryLock(lock.replace('1.53.1', '1.54.0'), [lock]), /registry drift/);
  assert.throws(() => auditRegistryLock(lock.replace('a'.repeat(64), 'b'.repeat(64)), [lock]), /registry drift/);
});

test('crate payload provenance admits a binary package and rejects omitted producer tests', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-ai-provenance-test-'));
  const producer = join(root, 'producer'), pkg = join(root, 'rom-cli-0.1.0');
  for (const base of [join(producer, 'crates/rom-cli'), pkg]) {
    mkdirSync(join(base, 'src'), { recursive: true }); mkdirSync(join(base, 'tests'));
    writeFileSync(join(base, 'Cargo.toml'), '[package]\nname = "rom-cli"\nversion = "0.1.0"\n');
    writeFileSync(join(base, 'src/main.rs'), 'fn main() {}'); writeFileSync(join(base, 'LICENSE'), 'MIT');
    writeFileSync(join(base, 'tests/public.rs'), '#[test] fn public() {}');
  }
  writeFileSync(join(pkg, 'Cargo.toml.orig'), readFileSync(join(producer, 'crates/rom-cli/Cargo.toml')));
  requirePackageProvenance([{ name: 'rom-cli', directory: pkg, files: inventory(pkg) }], producer);
  const files = inventory(pkg); delete files['tests/public.rs'];
  assert.throws(() => requirePackageProvenance([{ name: 'rom-cli', directory: pkg, files }], producer), /omitted/);
});

test('producer witness rejects a version claim that differs from actual workspace source', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-ai-producer-test-'));
  writeFileSync(join(root, 'Cargo.toml'), '[workspace.package]\nversion = "0.0.3"\n');
  writeFileSync(join(root, 'Cargo.lock'), 'version = 4\n');
  const files = ['Cargo.lock', 'Cargo.toml'].map(path => ({ path, mode: '100644', sha256: hash(join(root, path)) }));
  const source = { files, revision: 'a'.repeat(40), lock_sha256: hash(join(root, 'Cargo.lock')), source_inventory_sha256: digest(files), package_version: '0.0.3', source_mode: 'provisional-working-source' };
  fenceProducer(root, source);
  assert.throws(() => fenceProducer(root, { ...source, package_version: '0.1.0' }), /version/);
  assert.throws(() => fenceProducer(root, { ...source, source_mode: 'release' }), /witness/);
});

test('compile checker cwd receives the pinned package configuration and its complete input fence', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-ai-compile-bridge-test-')), source = join(root, 'fixture');
  mkdirSync(join(source, '.cargo'), { recursive: true }); mkdirSync(join(source, 'src'));
  writeFileSync(join(source, '.cargo/config.toml'), '[patch.crates-io]\nrom = { path = "/isolated/packages/rom" }\n');
  writeFileSync(join(source, 'Cargo.toml'), '[package]\nname = "fixture"\n');
  writeFileSync(join(source, 'Cargo.lock'), 'locked'); writeFileSync(join(source, 'check.mjs'), '// unchanged checker');
  const bridge = prepareCompileBridge(source, join(root, 'bridge'));
  assert.equal(readFileSync(join(bridge.app, '.cargo/config.toml'), 'utf8'), '[patch.crates-io]\nrom = { path = "/isolated/packages/rom" }\n');
  assert.equal(readFileSync(bridge.checker, 'utf8'), '// unchanged checker');
  fenceInputs(bridge.app, bridge.inputs);
  writeFileSync(join(bridge.app, '.cargo/config.toml'), 'changed');
  assert.throws(() => fenceInputs(bridge.app, bridge.inputs), /changed/);
});
