# Framework release scenario evidence

Date: 2026-10-03. Scope: accepted source revision `37c4f87cb4bd5d43975ed6ce1e049630a41d4f55` and the complete six-command release gate.

All 17 requirement names and 55 scenario names below match `openspec/changes/prepare-framework-release/specs/framework-readiness/spec.md`. The named tests are current maintained source. Report names refer to `docs/research/`. Historical execution is attributed to those reports, not claimed as this reviewer's rerun.

The [final release manifest](evidence/framework-release-2026-10-03/release/manifest.json) records six successful gates. Each row retains its direct-test or source-inspection qualification. Copied-application tests also passed against exclusively extracted ROM dependencies. Real-provider, process-exit, and author-workflow scopes remain separate. Signal and artifact acceptance are additional final-design obligations.

## Requirement: Application authors use one public Resource contract

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Pending compensation survives application restart | `demo/tests/reference.rs::committed_rejection_recovers_after_process_exit_without_shutdown` | `docs/research/framework-release-progress.md`<br>`docs/research/resource-transition-validation-results.md` | Final gates passed.  |
## Requirement: Data lifecycle preserves obligations

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Upgrade is interrupted | `tests/persistence/tests/upgrade.rs::native_upgrade_interruption_before_publication_is_retryable` | `docs/research/reference-upgrade-results.md`<br>`docs/research/runtime-ownership-results.md` | Final gates passed.  |
| Concurrent reference creation and target deletion | `tests/persistence/tests/references.rs::concurrent_adapter_create_delete_race_preserves_integrity` | `docs/research/restrict-reference-results.md`<br>`docs/research/runtime-ownership-results.md` | Final gates passed.  |
| A source kind is absent from a later Runtime | `tests/persistence/tests/references.rs::omitted_source_kind_after_reopen_cannot_erase_restrict` | `docs/research/restrict-reference-results.md` | Final gates passed.  |
| Receipt replay retains historical meaning | `tests/persistence/tests/references.rs::historical_replay_after_unlink_is_not_a_new_edge` | `docs/research/restrict-reference-results.md` | Final gates passed.  |
| A backup has an incomplete reference index | `tests/persistence/tests/backup.rs::backup_restore_preserves_restrict_edges_and_rejects_missing_edges` | `docs/research/restrict-reference-results.md` | Final gates passed.  |
| A Resource field changes representation | `tests/persistence/tests/schema_migration.rs::native_schema_migration_preserves_history_references_work_and_backup` | `docs/research/resource-migration-results.md` | Final gates passed.  |
| A request is retried after a field rename | `tests/persistence/tests/migration_runtime.rs::migrated_runtime_replays_old_input_and_resumes_both_reaction_phases`<br>`demo/tests/upgrade.rs::migrated_receipt_requires_its_recorded_codec_and_current_authority` | `docs/research/resource-migration-results.md`<br>`docs/research/reference-upgrade-results.md` | Final gates passed.  |
| Migration contains unfinished obligations | `tests/persistence/tests/schema_migration.rs::rejected_migration_never_publishes_or_modifies_source`<br>`demo/tests/upgrade.rs::invalid_conversion_missing_target_and_unaccepted_work_never_publish` | `docs/research/resource-migration-results.md`<br>`docs/research/reference-upgrade-results.md` | Final gates passed.  |
| An older maintenance tool reads a migrated store | `tests/persistence/tests/schema_migration.rs::legacy_catalogued_formats_require_explicit_upgrade_or_migration_without_source_writes` | `docs/research/resource-migration-results.md` | Final gates passed. Marker/metadata compatibility, not execution of an arbitrary obsolete binary. |
| Retention expires an idempotency epoch | `tests/persistence/tests/retention.rs::retention_expires_even_retained_anchors_and_survives_reopen_backup` | `docs/research/retention-results.md` | Final gates passed.  |
| Sealed work drains before expiry | `tests/persistence/tests/retry_epochs.rs::sealed_epoch_drains_nonzero_root_actions_and_notifications_without_fresh_admission`<br>`tests/persistence/tests/retention.rs::sealed_epoch_allows_replay_and_only_exact_causal_claim_for_fresh_work` | `docs/research/retention-results.md` | Final gates passed.  |
| Old backup predates a retention policy | `tests/persistence/tests/retry_epochs.rs::trusted_fence_rejects_an_older_restored_backup` | `docs/research/retention-results.md` | Final gates passed.  |
| The reference application upgrades with pending compensation | `demo/tests/upgrade.rs::process_exit_preserves_pending_compensation_through_migration_backup_and_restore` | `docs/research/reference-upgrade-results.md` | Final gates passed. Packaged execution passed; source preservation, unrelated reservation, attachment, replay and restrict checks live in the called public recovery journey. |
## Requirement: Optimization preserves observable semantics

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Index recovery changes execution path | `tests/persistence/tests/sqlite_query_index.rs::backup_restore_and_explicit_rebuild_preserve_logical_data_and_fence_indexes`<br>`tests/persistence/tests/query_read.rs::reference_and_complete_candidates_share_typed_projected_and_live_results` | `docs/research/query-integration-foundation-results.md`<br>`docs/research/query-read-contract-results.md` | Final gates passed.  |
| An excluded row has a protected sort field | `tests/persistence/tests/query_planning.rs::sorted_field_denial_precedes_predicates_anchor_and_page_limit` | `docs/research/query-integration-foundation-results.md`<br>`docs/research/query-read-contract-results.md` | Final gates passed.  |
| An opaque callback fails before filtering | `tests/persistence/tests/query_planning.rs::opaque_row_policy_runs_before_a_nonmatching_predicate_can_skip_the_row`<br>`tests/persistence/tests/query_planning.rs::id_page_short_circuit_does_not_evaluate_later_opaque_policy` | `docs/research/query-integration-foundation-results.md` | Final gates passed.  |
| Planning uses coherent admission metadata | `crates/rom-sqlite/src/index/query_tests.rs::real_index_returns_complete_candidates_and_exact_binding`<br>`crates/rom-sqlite/src/index/query_tests.rs::planner_failure_falls_back_but_selected_execution_failure_propagates` | `docs/research/query-read-contract-results.md` | Final gates passed. Coherent transaction is also inspected in the adapter implementation; estimates are optional, execution failure is not. |
| Application policy can read storage | `tests/persistence/tests/query_read.rs::reference_and_complete_candidates_share_typed_projected_and_live_results` | `docs/research/query-read-contract-results.md` | Final gates passed. Structural inspection: query adapter accepts no application callbacks; query_eval/read runs policies/codecs after native read returns. No dedicated reentrant Storage-read policy test located; do not claim one. |
| Explicit uniform read authorization | `tests/persistence/tests/query_read.rs::explicit_denial_precedes_empty_or_overbound_storage_reads`<br>`tests/persistence/tests/query_read.rs::read_grant_runs_once_after_query_validation_and_disclosure_rechecks_authority`<br>`tests/persistence/tests/query_read.rs::excluded_decoder_is_skipped_only_under_explicit_uniform_contract` | `docs/research/query-read-contract-results.md` | Final gates passed.  |
| A later definition clears optimization permission | `tests/persistence/tests/query_read.rs::builder_overrides_keep_opaque_selection_and_field_preflight` | `docs/research/query-read-contract-results.md` | Final gates passed.  |
| Adapter returns candidates for a different request | `tests/persistence/tests/query_read.rs::malformed_native_responses_and_execution_failures_never_fall_back`<br>`tests/persistence/tests/query_read.rs::native_candidates_cannot_bypass_whole_kind_admission_with_a_small_page` | `docs/research/query-read-contract-results.md` | Final gates passed.  |
| Cost estimates cannot authorize a native path | `crates/rom/src/query_storage/tests.rs::selector_requires_exact_capability_binding_and_strict_checked_saving`<br>`crates/rom/src/query_storage/tests.rs::selector_rejects_opaque_unknown_shapes_versions_and_identity`<br>`crates/rom/src/query_storage/tests.rs::checked_score_rejects_addition_overflow_and_unrelated_snapshot_identity` | `docs/research/query-integration-foundation-results.md` | Final gates passed.  |
| An indexed mutation rolls back or replays | `tests/persistence/tests/sqlite_query_index.rs::native_commit_replay_rollback_delete_and_reopen_are_coherent` | `docs/research/query-integration-foundation-results.md` | Final gates passed.  |
| A derived index is corrupt | `tests/persistence/tests/sqlite_query_index.rs::backup_restore_and_explicit_rebuild_preserve_logical_data_and_fence_indexes`<br>`tests/persistence/tests/sqlite_index_recovery.rs::rebuilding_derived_data_cannot_repair_missing_authoritative_receipt_proof`<br>`tests/persistence/tests/sqlite_index_recovery.rs::logical_archive_fits_but_combined_native_index_budget_blocks_publication` | `docs/research/query-integration-foundation-results.md` | Final gates passed.  |
| Maintenance reconstructs an index | `tests/persistence/tests/sqlite_index_recovery.rs::omitted_catalog_kinds_survive_registration_and_index_rebuild`<br>`tests/persistence/tests/sqlite_index_recovery.rs::process_exit_leaves_source_unchanged_and_no_published_index_rebuild`<br>`tests/persistence/tests/sqlite_query_index.rs::backup_restore_and_explicit_rebuild_preserve_logical_data_and_fence_indexes` | `docs/research/query-integration-foundation-results.md`<br>`docs/research/maintained-query-cost-results.md` | Final gates passed.  |
| The previous native format has retry epochs | `tests/persistence/tests/sqlite_query_index.rs::format_six_upgrade_preserves_retry_epochs_and_does_not_rebind_receipt_origins`<br>`crates/rom-backup/src/epoch_upgrade_tests.rs::archive_four_requires_explicit_upgrade_and_preserves_epochs_and_origins` | `docs/research/query-integration-foundation-results.md`<br>`docs/research/operator-recovery-results.md` | Final gates passed.  |
## Requirement: Query strategy probes preserve the Resource contract

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| A broad predicate precedes a selective predicate | `crates/rom-sqlite/src/index/planner/tests.rs::conjunction_order_does_not_hide_the_eight_row_candidate`<br>`crates/rom-sqlite/src/index/planner/tests.rs::saturated_probe_does_not_truncate_forced_native_materialization` | `docs/research/maintained-query-cost-results.md` | Final gates passed.  |
| Probe work reaches its configured bound | `crates/rom-sqlite/src/index/planner/tests.rs::probes_distinguish_exact_counts_saturation_and_reference_control`<br>`crates/rom-sqlite/src/index/planner/tests.rs::expanded_probe_shrinks_against_an_exact_candidate_and_keeps_total_work_bounded` | `docs/research/maintained-query-cost-results.md` | Final gates passed.  |
| Native execution is already ineligible | `crates/rom-sqlite/src/index/planner/tests.rs::empty_candidate_stops_probing_and_opaque_reads_do_not_probe` | `docs/research/maintained-query-cost-results.md` | Final gates passed.  |
## Requirement: Runtime ownership excludes competing supervisors

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| A wrapper exposes an already owned store | `crates/rom/tests/storage_ownership.rs::storage_ownership_excludes_wrapped_and_direct_builders_before_storage_reads`<br>`examples/consumer/tests/lifecycle.rs::ownership_transparent_wrapper_forwards_underlying_claim` | `docs/research/runtime-ownership-results.md` | Final gates passed.  |
| A caller stops waiting for accepted work | `examples/consumer/tests/lifecycle.rs::ownership_cancelled_action_and_drain_waiter_keep_accepted_work_alive`<br>`crates/rom/tests/storage_ownership.rs::storage_ownership_shutdown_retains_claim_until_last_runtime_drops` | `docs/research/runtime-ownership-results.md` | Final gates passed.  |
| Construction fails after acquisition | `crates/rom/tests/storage_ownership.rs::storage_ownership_failed_registration_releases_claim` | `docs/research/runtime-ownership-results.md` | Final gates passed.  |
## Requirement: Native ownership covers open and offline maintenance

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| A process exits with committed data | `tests/persistence/tests/ownership/process.rs::native_owner_process_exit_releases_lock_and_recovers_committed_data` | `docs/research/runtime-ownership-results.md` | Final gates passed.  |
| A database has an alias | `tests/persistence/tests/ownership.rs::symlink_alias_cannot_create_a_second_native_owner`<br>`tests/persistence/tests/ownership.rs::hardlinked_database_is_rejected_before_native_open`<br>`tests/persistence/tests/ownership.rs::sqlite_memory_sentinel_keeps_stores_independent` | `docs/research/runtime-ownership-results.md` | Final gates passed.  |
| A conversion callback attempts a concurrent open | `tests/persistence/tests/ownership/maintenance.rs::conversion_holds_both_reservations_and_failure_releases_them`<br>`tests/persistence/tests/ownership/offline_recovery.rs::callback_cwd_change_keeps_native_maintenance_paths_canonical` | `docs/research/runtime-ownership-results.md` | Final gates passed.  |
| Publication is interrupted before staging cleanup | `crates/rom-backup/src/publication/tests.rs::native_publication_exit_after_link_requires_exact_artifact_recovery`<br>`crates/rom-backup/src/publication/tests.rs::native_finalization_refuses_unknown_additional_hard_links` | `docs/research/runtime-ownership-results.md` | Final gates passed.  |
## Requirement: Release readiness includes operational recovery

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Operator recovers a reference deployment | `demo/tests/operator.rs::operator_command_recovers_work_then_runs_explicit_compensation_on_both_stores`<br>`demo/tests/upgrade.rs::upgrade_command_runs_the_real_application_on_both_backends`<br>`demo/tests/reference.rs::committed_rejection_recovers_after_process_exit_without_shutdown` | `docs/research/operator-recovery-results.md`<br>`docs/research/reference-upgrade-results.md` | Final gates passed. Packaged reference execution and path audit passed. Explicit synthetic host identity; real provider journey remains separate. |
## Requirement: Operator work views use current authority and bounded projections

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Authority changes between pages | `tests/persistence/tests/operator_runtime.rs::moving_page_rechecks_scope_permission_after_anchor_revocation`<br>`tests/persistence/tests/operator_runtime.rs::exact_control_replay_rechecks_current_operator_and_actor_authority` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
| The view cursor refers to an older storage generation | `tests/persistence/tests/operator_runtime.rs::raw_snapshot_bounds_and_generation_are_checked_before_projection`<br>`tests/persistence/tests/operator_runtime.rs::response_limits_refuse_new_controls_before_commit_and_preserve_unknown_for_committed_replays` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
## Requirement: Operator controls have atomic version and identity arbitration

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Delivery changes without a new claim | `crates/rom/src/reaction_work/revision_tests.rs::delivery_updates_increment_once_and_preserve_claim_generation`<br>`crates/rom/src/reaction_work/revision_tests.rs::revision_overflow_rejects_claim_without_consuming_budget`<br>`tests/persistence/tests/operator.rs::native_control_cas_and_exact_replay_preserve_domain_state_and_budgets` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
| A control commits but its response is lost | `crates/rom-cli/tests/operator.rs::lost_control_reply_replays_the_exact_file_only_after_an_explicit_new_command`<br>`tests/persistence/tests/operator_runtime.rs::exact_control_replay_rechecks_current_operator_and_actor_authority`<br>`tests/persistence/tests/operator.rs::native_control_cas_and_exact_replay_preserve_domain_state_and_budgets` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
| An operator requests another attempt | `tests/persistence/tests/operator.rs::native_control_cas_and_exact_replay_preserve_domain_state_and_budgets`<br>`tests/persistence/tests/reactions.rs::persisted_work_lease_and_input_are_fenced`<br>`tests/persistence/tests/reactions.rs::finite_work_budget_allows_receipt_resolution_but_no_new_execution` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
## Requirement: Delivery reconciliation preserves declared guarantees

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| An uncertain delivery uses the hold profile | `tests/persistence/tests/operator_reconciliation/recovery.rs::actual_process_exit_after_delivery_start_cannot_trigger_unsafe_resend`<br>`tests/persistence/tests/operator_reconciliation/races.rs::current_operator_actor_and_service_revocation_are_rechecked_after_verification`<br>`tests/persistence/tests/operator_reconciliation/races.rs::stale_work_version_after_verification_cannot_publish_a_second_receipt` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
| A provider lookup is inconclusive | `tests/persistence/tests/operator_reconciliation/verification.rs::unresolved_eventual_lookup_miss_cannot_grant_resend_before_later_acceptance`<br>`tests/persistence/tests/operator_reconciliation/verification.rs::accepted_and_terminal_not_accepted_commit_once_without_a_provider_send`<br>`tests/persistence/tests/operator_reconciliation/verification.rs::terminal_not_accepted_cannot_reset_exhausted_attempts_or_root_budget` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
| The application keeps its existing delivery profile | `tests/persistence/tests/operator_reconciliation/profiles.rs::all_declared_profiles_preserve_their_uncertain_delivery_policy`<br>`crates/rom/src/reaction_work/delivery_profile_tests.rs::default_profile_plain_retry_keeps_existing_unknown_delivery_policy` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
## Requirement: Operator CLI preserves generic Resource semantics

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| A server returns valid JSON for another work item | `crates/rom-cli/tests/operator.rs::control_responses_match_request_identity_operation_and_version_transition`<br>`crates/rom-cli/tests/operator/signals.rs::control_interrupt_does_not_wait_on_blocked_stdout_or_stderr` | `docs/research/operator-recovery-results.md` | Final gates passed.  |
| Maintenance preserves operator evidence | `crates/rom-backup/src/epoch_upgrade_tests.rs::archive_restore_and_retention_preserve_historical_operator_receipts`<br>`crates/rom-backup/src/epoch_upgrade_tests.rs::current_archive_and_restore_preserve_reconciliation_hold_and_profile` | `docs/research/operator-recovery-results.md` | Final gates passed. Shared snapshot archive/restore/retention tests are direct; native migration preservation also uses inspected shared snapshot state-copy/validation. Do not label this a distinct native migration seeded-operator test. |
## Requirement: A release identity profile uses real provider evidence

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| An actual service token reaches an ordinary Resource | `demo/provider-fixture/journey.mjs::resourceJourney`<br>`demo/provider-fixture/journey.mjs::identityJourney` | `docs/research/provider-deployment-results.md` | Final gates passed. Actual Node/CLI pinned-provider acceptance; unlinked/expiry/binding/rotation denial cases are in journey. Separate command ./demo/verify-provider; final source execution passed. |
## Requirement: Authentication acquisition has bounded owned execution

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| A caller leaves during blocking verification | `demo/tests/provider_auth/admission.rs::caller_cancellation_retains_capacity_and_cancelled_drain_can_be_reawaited`<br>`demo/tests/provider_auth/admission.rs::caller_deadline_retains_capacity_until_verifier_completion` | `docs/research/provider-deployment-results.md` | Final gates passed.  |
## Requirement: Provider configuration cannot grant arbitrary acquisition authority

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Configuration changes while evidence is obtained | `demo/tests/provider_auth/admission.rs::provider_revision_change_while_verifying_denies_stale_binding_without_holding_core_gate` | `docs/research/provider-deployment-results.md` | Final gates passed.  |
## Requirement: Local provisioning is explicit and restartable

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| Provisioning stops between Resource commits | `demo/tests/provider_profile/provisioning.rs::each_lost_ack_step_resumes_after_native_close_and_reopen`<br>`demo/tests/provider_profile/provisioning.rs::both_store_resume_preserves_revisions_and_changed_input_is_rejected` | `docs/research/provider-deployment-results.md` | Final gates passed.  |
## Requirement: Native extension releases expose versioned conformance

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| An external adapter runs the baseline profile | `examples/consumer/tests/native_conformance.rs::external_fixture_passes_storage_profile_and_releases_owners_before_reopen`<br>`examples/consumer/tests/native_conformance.rs::broken_fixture_fails_named_counts_assertion`<br>`tests/persistence/tests/shared.rs::shared_atomic_bundle_and_noop_identity_validation` | `docs/research/native-conformance-results.md` | Final gates passed. Maintained callers share the public runner; engine-specific interruption cases remain additional acceptance. |
| An author selects an obsolete profile | `examples/consumer/tests/native_conformance.rs::wrong_profile_fails_before_factory_operations`<br>`scripts/skills/bundle.test.mjs::command rejects incompatible profile feature and missing asset before executing any example` | `docs/research/native-conformance-results.md` | Final gates passed.  |
## Requirement: Author skills have portable executable evidence

| Scenario | Current maintained case or inspected mechanism | Historical evidence | Final status / limits |
| --- | --- | --- | --- |
| A skill is extracted outside the development checkout | `scripts/skills/bundle.test.mjs::portable bundle has deterministic content and verifies outside its source directory`<br>`.superpowers/sdd/2026-10-03-native-conformance/skill-evaluation/report.md` | `docs/research/native-conformance-results.md` | Final gates passed. Independent held-out author tasks executed; evaluator familiarity retained; full source-checkout prerequisite explicit. |

## Structural evidence qualifications

“Application policy can read storage” is supported by the adapter boundary and read/evaluation order inspected in `crates/rom/src/query_eval/read.rs` and the native `query_read` implementation. The adapter receives no application callback. Current observation tests exercise policy ordering and generation/authority rechecks. This audit did not locate a dedicated callback that reenters native Storage in that suite. Do not turn the structural evidence into a claimed executed reentrant-policy test.

“Maintenance preserves operator evidence” has direct shared-snapshot archive/restore/retention and strict profile-validation tests. Native schema migration also retains and validates the shared state object; this is inspected implementation evidence. The mapping does not claim a separately authored native migration case seeded with operator receipts.

## Completed final-design acceptance outside the historical mapping

- Task1: shared pre-readiness signal registration, immediate SIGINT/SIGTERM in both server modes, bounded child ownership, and completed attachment reopen. Shared shutdown source and existing paused-auth/blob component tests provide separate drain evidence.
- Task2: copy the real reference application, preserve dependency/feature/package identity, run all applicable maintained tests against fresh archives, and audit resolved ROM manifests. Keep actual-provider acceptance separate.
- Task3: clean-source admission, exclusive output, all required commands, failure staging, archive/bundle/checksum/manifest identity, extraction with matching skill admission, and no publication.

The [completion audit](framework-release-completion.md) binds final source and lock identities, package evidence, and artifact checks. Historical reports and prototypes remain unchanged.
