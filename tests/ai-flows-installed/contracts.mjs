// Installed-consumer authoring and executed-output acceptance contracts.
export function rewriteAiManifest(text, packages) {
  if (/^\s*\[(?:patch|replace)[.\]]/m.test(text) || /git\s*=/.test(text)) throw Error('unreviewed source override');
  let count = 0;
  const result = text.split('\n').map(line => {
    const match = /^(rom(?:-[a-z-]+)?)\s*=\s*\{([^}]+)\}\s*$/.exec(line);
    if (!match) return line;
    const pkg = packages.find(pkg => pkg.name === match[1]);
    if (!pkg) throw Error('missing declared extracted package');
    const path = /\bpath\s*=\s*"([^"]+)"/.exec(match[2]);
    if (path && path[1] !== `../../crates/${pkg.name}`) throw Error('unreviewed dependency path');
    const options = match[2].split(/,\s*(?=[a-z_-]+\s*=)/).map(value => value.trim());
    const preserved = options.filter(value => !/^(?:path|version)\s*=/.test(value));
    if (preserved.some(value => !/^(?:features|default-features)\s*=/.test(value))) throw Error('unreviewed dependency options');
    count++;
    return `${pkg.name} = { version = "=${pkg.version}"${preserved.length ? ', ' + preserved.join(', ') : ''} }`;
  }).join('\n');
  if (!count || /\bpath\s*=\s*"\.\.\//.test(result)) throw Error('retained checkout dependency path');
  return result;
}

const domains = [
  'cancellation::sqlite_pending_successor_cancellation_preserves_hold_after_reopen',
  'cancellation::redb_pending_successor_cancellation_preserves_hold_after_reopen',
  'regressions::sqlite_prepared_expiry_in_preflight_fails_and_releases_after_reopen',
  'regressions::redb_prepared_expiry_in_preflight_fails_and_releases_after_reopen',
  'regressions::sqlite_prepared_attempt_only_denial_is_public_terminal',
  'regressions::redb_prepared_attempt_only_denial_is_public_terminal',
  'regressions::sqlite_overlapping_prepared_callbacks_dispatch_alternate_once',
  'regressions::redb_overlapping_prepared_callbacks_dispatch_alternate_once',
  'regressions::sqlite_terminal_tombstone_fences_late_uncommitted_reserve',
  'regressions::redb_terminal_tombstone_fences_late_uncommitted_reserve',
  'sqlite_completed_tool_grant_revocation_is_terminal',
  'redb_completed_tool_grant_revocation_is_terminal',
  'sqlite_failover_exact_reference_survives_due_and_reopen',
  'redb_failover_exact_reference_survives_due_and_reopen',
  'sqlite_failover_generation_is_held_without_zero_settlement',
  'redb_failover_generation_is_held_without_zero_settlement',
  'sqlite_failover_uncertain_known_cost_stays_charged_without_retry',
  'redb_failover_uncertain_known_cost_stays_charged_without_retry',
  'sqlite_failover_lost_waiting_ack_preserves_receipt_and_single_successor',
  'redb_failover_lost_waiting_ack_preserves_receipt_and_single_successor',
  'sqlite_failover_untrusted_usage_retains_reserved_ceiling_after_reopen',
  'redb_failover_untrusted_usage_retains_reserved_ceiling_after_reopen',
  'sqlite_failover_original_deadline_before_rounded_due_never_retransmits',
  'redb_failover_original_deadline_before_rounded_due_never_retransmits',
  'sqlite_failover_current_grant_revoked_before_retry_never_retransmits',
  'redb_failover_current_grant_revoked_before_retry_never_retransmits',
  'sqlite_failover_two_unrelated_runs_have_independent_durable_cursors',
  'redb_failover_two_unrelated_runs_have_independent_durable_cursors',
  'sqlite_failover_exact_commits_recover_lost_acknowledgements',
  'redb_failover_exact_commits_recover_lost_acknowledgements',
  'sqlite_failover_rejects_forged_or_rewritten_persisted_decisions',
  'redb_failover_rejects_forged_or_rewritten_persisted_decisions',
  'sqlite_failover_positive_reconciliation_preserves_known_charge',
  'redb_failover_positive_reconciliation_preserves_known_charge',
  'sqlite_failover_exhaustion_is_durable_and_keeps_original_evidence',
  'redb_failover_exhaustion_is_durable_and_keeps_original_evidence',
  'sqlite_failover_known_cost_counts_before_successor_reservation',
  'redb_failover_known_cost_counts_before_successor_reservation',
  'sqlite_failover_fresh_catalog_rejects_now_overpriced_alternate',
  'redb_failover_fresh_catalog_rejects_now_overpriced_alternate',
  'sqlite_publication_completed_tools_survive_model_failover',
  'redb_publication_completed_tools_survive_model_failover',
  'sqlite_triage_completed_tools_survive_model_failover',
  'redb_triage_completed_tools_survive_model_failover',
  'sqlite_public_publication_flow_prepares_then_publishes_with_retained_sources',
  'redb_public_publication_flow_prepares_then_publishes_with_retained_sources',
  'sqlite_public_ticket_triage_flow_reads_then_classifies_once',
  'redb_public_ticket_triage_flow_reads_then_classifies_once',
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
export function requireAiResults(text, kind) {
  const rows = [...text.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/gm)];
  if (!rows.length) throw Error('missing native test results');
  if (rows.some(row => row[1] !== 'ok' || Number(row[3]) || Number(row[4]))) throw Error('failed or ignored native acceptance');
  const passed = rows.reduce((sum, row) => sum + Number(row[2]), 0);
  if (!passed) throw Error('empty native test acceptance');
  if (kind === 'flows') for (const name of domains) {
    if (!text.split('\n').includes(`test ${name} ... ok`)) throw Error(`missing executed domain test: ${name}`);
  }
  if (kind === 'adapter') for (const name of ['sqlite_actual_http_flow_selects_alternate_eligible_endpoint', 'redb_actual_http_flow_selects_alternate_eligible_endpoint']) {
    if (!text.split('\n').includes(`test ${name} ... ok`)) throw Error(`missing feature-enabled adapter test: ${name}`);
  }
  return { passed, failed: 0, ignored: 0 };
}

export function requireAiCompiler(text, expected) {
  const release = /^release: (.+)$/m.exec(text)?.[1];
  const host = /^host: (.+)$/m.exec(text)?.[1];
  if (release !== expected || !host || !text.startsWith(`rustc ${expected} `)) throw Error('compiler identity mismatch');
  return { release, host, verbose: text };
}

export function validateOptions(options) {
  for (const key of ['metadataSeconds', 'testSeconds', 'compileSeconds', 'outputBytes']) {
    if (!Number.isSafeInteger(options?.[key]) || options[key] < 1) throw Error(`explicit finite budget required: ${key}`);
  }
  for (const key of ['metadataSeconds', 'testSeconds', 'compileSeconds']) if (options[key] > 600) throw Error('command budget exceeds 10 minutes');
  if (options.outputBytes > 8 * 1024 * 1024) throw Error('output budget exceeds 8 MiB');
  if (!/^\d+\.\d+\.\d+$/.test(options.compiler ?? '')) throw Error('explicit compiler identity required');
}
