# Final real-producer execution audit

Status: completed source-only release acceptance confirmed. Spec: accepted. Quality: accepted. No findings.

The successful real producer executed clean-main commit `37c4f87cb4bd5d43975ed6ce1e049630a41d4f55`. All six commands passed. The completed local output is `/root/ROM/dist/rom-0.1.0-alpha.1-37c4f87cb4bd`. It contains source and skills archives, a complete manifest, twelve gate logs, and checksums. External publication is disabled.

Independent read-only review checked all 15 checksum entries and compared the complete clean-main source snapshot with the manifest. All 1014 file hashes and modes, Git tree, profile, selected inventory and lock identity match. The tree is `d86a81028e887c25e18738c11e1a66c19709ca31`. The lock SHA-256 is `fd8c304a3bde42c05c9503f0c6de201bd23b2d788dafba678c508ebae91b9a83`. The coordinator’s absolute-path artifact verification returned success and checked extraction, source/tree/profile/selected identity and four bundle preflights. This reviewer inspected that result; broad gates and complete extraction were not repeated.

The earlier producer on `6d59c938a796640b1d64798eb51a8c187b091864` passed all six gates but failed post-gate with EPIPE. Its stage and review remain preserved. The accepted two-file repair changed only the Git parser input to the required header prefix and added large-archive and corruption regressions. The final producer reran every mandatory command. The coordinator’s initial relative-path private verifier call failed path lookup; the corrected absolute-path call succeeded without artifact or source changes.

## Actual command partitions

| Gate | Executed command | Observed result and boundary |
| --- | --- | --- |
| 1 | ./scripts/check | Exit0: all9 OpenSpec changes validate; package/artifact Node tests and full Rust floor/format/Clippy/tests/docs/negative compile/auth/identity checks passed. Maintained native, query, operator, ownership and default application tests execute here. |
| 2 | ./scripts/build --release | Exit0: optimized build completed. Compilation is not a behavioral scenario. |
| 3 | ./demo/verify | Exit0: default reference application tests, both-store smoke, serve/reopen/SIGINT and docs checks passed. Native upgrade/process-exit/attachment/authority assertions are retained in called application cases. |
| 4 | ./demo/verify-provider | Exit0: opt-in host-auth/profile suites and actual pinned provider fixtures/journeys passed. Output explicitly closes operations, receipt replay, ROM reopen, access revocation, audience binding, credential rotation and expiry/fresh-token/marker scans on both stores. This is separate from synthetic upgrade/operator journeys. |
| 5 | ./scripts/check-skills | Exit0: seven Node tests, relocated assembly and four extracted examples passed. Earlier independent held-out author tasks remain separate historical evidence; they were not newly rerun here. |
| 6 | node scripts/check-packages.mjs /workspace/ROM | Exit0:13fresh archives, external consumer, extracted CLI, full copied demo all-feature tests and final resolved-path audit passed. This is the actual package run, separate from prior failed attempts. |

## Exact55 scenario mapping

Every requirement/scenario name below matches the release specification. Numbers identify the actual command partitions above. Source reference names identify test bodies or their inspected shared mechanisms. The observed test names are current log matches, with macro-generated wrappers recorded for the three reference helpers. Historical reports remain attributed in scenario-map.md.

### Application authors use one public Resource contract

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Pending compensation survives application restart | 1, 3, 6; `committed_rejection_recovers_after_process_exit_without_shutdown` |  |
### Data lifecycle preserves obligations

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Upgrade is interrupted | 1; `native_upgrade_interruption_before_publication_is_retryable` |  |
| Concurrent reference creation and target deletion | 1; `sqlite::competing_adapter_create_delete_cannot_commit_dangling_reference`<br>`redb::competing_adapter_create_delete_cannot_commit_dangling_reference` |  |
| A source kind is absent from a later Runtime | 1; `sqlite::omitted_kind_and_changed_schema_preserve_persisted_integrity`<br>`redb::omitted_kind_and_changed_schema_preserve_persisted_integrity` |  |
| Receipt replay retains historical meaning | 1; `sqlite::old_receipt_replay_after_unlink_does_not_add_an_edge`<br>`redb::old_receipt_replay_after_unlink_does_not_add_an_edge` |  |
| A backup has an incomplete reference index | 1; `backup_restore_preserves_restrict_edges_and_rejects_missing_edges` |  |
| A Resource field changes representation | 1; `native_schema_migration_preserves_history_references_work_and_backup` |  |
| A request is retried after a field rename | 1; `migrated_runtime_replays_old_input_and_resumes_both_reaction_phases`<br>`migrated_receipt_requires_its_recorded_codec_and_current_authority` |  |
| Migration contains unfinished obligations | 1; `rejected_migration_never_publishes_or_modifies_source`<br>`invalid_conversion_missing_target_and_unaccepted_work_never_publish` |  |
| An older maintenance tool reads a migrated store | 1; `legacy_catalogued_formats_require_explicit_upgrade_or_migration_without_source_writes` | Marker/metadata compatibility, not execution of an arbitrary obsolete binary. |
| Retention expires an idempotency epoch | 1; `retention_expires_even_retained_anchors_and_survives_reopen_backup` |  |
| Sealed work drains before expiry | 1; `sealed_epoch_drains_nonzero_root_actions_and_notifications_without_fresh_admission`<br>`sealed_epoch_allows_replay_and_only_exact_causal_claim_for_fresh_work` |  |
| Old backup predates a retention policy | 1; `trusted_fence_rejects_an_older_restored_backup` |  |
| The reference application upgrades with pending compensation | 1, 3, 6; `process_exit_preserves_pending_compensation_through_migration_backup_and_restore` | Actual packaged execution passed in gate6; source preservation, unrelated reservation, attachment, replay and restrict checks live in the called public recovery journey. |
### Optimization preserves observable semantics

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Index recovery changes execution path | 1; `backup_restore_and_explicit_rebuild_preserve_logical_data_and_fence_indexes`<br>`reference_and_complete_candidates_share_typed_projected_and_live_results` |  |
| An excluded row has a protected sort field | 1; `sorted_field_denial_precedes_predicates_anchor_and_page_limit` |  |
| An opaque callback fails before filtering | 1; `opaque_row_policy_runs_before_a_nonmatching_predicate_can_skip_the_row`<br>`id_page_short_circuit_does_not_evaluate_later_opaque_policy` |  |
| Planning uses coherent admission metadata | 1; `real_index_returns_complete_candidates_and_exact_binding`<br>`planner_failure_falls_back_but_selected_execution_failure_propagates` | Coherent transaction is also inspected in the adapter implementation; estimates are optional, execution failure is not. |
| Application policy can read storage | 1; `reference_and_complete_candidates_share_typed_projected_and_live_results` | Structural inspection: query adapter accepts no application callbacks; query_eval/read runs policies/codecs after native read returns. No dedicated reentrant Storage-read policy test located; do not claim one. |
| Explicit uniform read authorization | 1; `explicit_denial_precedes_empty_or_overbound_storage_reads`<br>`read_grant_runs_once_after_query_validation_and_disclosure_rechecks_authority`<br>`excluded_decoder_is_skipped_only_under_explicit_uniform_contract` |  |
| A later definition clears optimization permission | 1; `builder_overrides_keep_opaque_selection_and_field_preflight` |  |
| Adapter returns candidates for a different request | 1; `malformed_native_responses_and_execution_failures_never_fall_back`<br>`native_candidates_cannot_bypass_whole_kind_admission_with_a_small_page` |  |
| Cost estimates cannot authorize a native path | 1; `selector_requires_exact_capability_binding_and_strict_checked_saving`<br>`selector_rejects_opaque_unknown_shapes_versions_and_identity`<br>`checked_score_rejects_addition_overflow_and_unrelated_snapshot_identity` |  |
| An indexed mutation rolls back or replays | 1; `native_commit_replay_rollback_delete_and_reopen_are_coherent` |  |
| A derived index is corrupt | 1; `backup_restore_and_explicit_rebuild_preserve_logical_data_and_fence_indexes`<br>`rebuilding_derived_data_cannot_repair_missing_authoritative_receipt_proof`<br>`logical_archive_fits_but_combined_native_index_budget_blocks_publication` |  |
| Maintenance reconstructs an index | 1; `omitted_catalog_kinds_survive_registration_and_index_rebuild`<br>`process_exit_leaves_source_unchanged_and_no_published_index_rebuild`<br>`backup_restore_and_explicit_rebuild_preserve_logical_data_and_fence_indexes` |  |
| The previous native format has retry epochs | 1; `format_six_upgrade_preserves_retry_epochs_and_does_not_rebind_receipt_origins`<br>`archive_four_requires_explicit_upgrade_and_preserves_epochs_and_origins` |  |
### Query strategy probes preserve the Resource contract

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| A broad predicate precedes a selective predicate | 1; `conjunction_order_does_not_hide_the_eight_row_candidate`<br>`saturated_probe_does_not_truncate_forced_native_materialization` |  |
| Probe work reaches its configured bound | 1; `probes_distinguish_exact_counts_saturation_and_reference_control`<br>`expanded_probe_shrinks_against_an_exact_candidate_and_keeps_total_work_bounded` |  |
| Native execution is already ineligible | 1; `empty_candidate_stops_probing_and_opaque_reads_do_not_probe` |  |
### Runtime ownership excludes competing supervisors

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| A wrapper exposes an already owned store | 1; `storage_ownership_excludes_wrapped_and_direct_builders_before_storage_reads`<br>`ownership_transparent_wrapper_forwards_underlying_claim` |  |
| A caller stops waiting for accepted work | 1; `ownership_cancelled_action_and_drain_waiter_keep_accepted_work_alive`<br>`storage_ownership_shutdown_retains_claim_until_last_runtime_drops` |  |
| Construction fails after acquisition | 1; `storage_ownership_failed_registration_releases_claim` |  |
### Native ownership covers open and offline maintenance

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| A process exits with committed data | 1; `native_owner_process_exit_releases_lock_and_recovers_committed_data` |  |
| A database has an alias | 1; `symlink_alias_cannot_create_a_second_native_owner`<br>`hardlinked_database_is_rejected_before_native_open`<br>`sqlite_memory_sentinel_keeps_stores_independent` |  |
| A conversion callback attempts a concurrent open | 1; `conversion_holds_both_reservations_and_failure_releases_them`<br>`callback_cwd_change_keeps_native_maintenance_paths_canonical` |  |
| Publication is interrupted before staging cleanup | 1; `native_publication_exit_after_link_requires_exact_artifact_recovery`<br>`native_finalization_refuses_unknown_additional_hard_links` |  |
### Release readiness includes operational recovery

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Operator recovers a reference deployment | 1, 3, 6; `operator_command_recovers_work_then_runs_explicit_compensation_on_both_stores`<br>`upgrade_command_runs_the_real_application_on_both_backends`<br>`committed_rejection_recovers_after_process_exit_without_shutdown` | Actual packaged execution/path audit passed in gate6. Explicit synthetic host identity; real provider journey remains separate. |
### Operator work views use current authority and bounded projections

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Authority changes between pages | 1; `moving_page_rechecks_scope_permission_after_anchor_revocation`<br>`exact_control_replay_rechecks_current_operator_and_actor_authority` |  |
| The view cursor refers to an older storage generation | 1; `raw_snapshot_bounds_and_generation_are_checked_before_projection`<br>`response_limits_refuse_new_controls_before_commit_and_preserve_unknown_for_committed_replays` |  |
### Operator controls have atomic version and identity arbitration

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Delivery changes without a new claim | 1; `delivery_updates_increment_once_and_preserve_claim_generation`<br>`revision_overflow_rejects_claim_without_consuming_budget`<br>`native_control_cas_and_exact_replay_preserve_domain_state_and_budgets` |  |
| A control commits but its response is lost | 1; `lost_control_reply_replays_the_exact_file_only_after_an_explicit_new_command`<br>`exact_control_replay_rechecks_current_operator_and_actor_authority`<br>`native_control_cas_and_exact_replay_preserve_domain_state_and_budgets` |  |
| An operator requests another attempt | 1; `native_control_cas_and_exact_replay_preserve_domain_state_and_budgets`<br>`persisted_work_lease_and_input_are_fenced`<br>`finite_work_budget_allows_receipt_resolution_but_no_new_execution` |  |
### Delivery reconciliation preserves declared guarantees

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| An uncertain delivery uses the hold profile | 1; `actual_process_exit_after_delivery_start_cannot_trigger_unsafe_resend`<br>`current_operator_actor_and_service_revocation_are_rechecked_after_verification`<br>`stale_work_version_after_verification_cannot_publish_a_second_receipt` |  |
| A provider lookup is inconclusive | 1; `unresolved_eventual_lookup_miss_cannot_grant_resend_before_later_acceptance`<br>`accepted_and_terminal_not_accepted_commit_once_without_a_provider_send`<br>`terminal_not_accepted_cannot_reset_exhausted_attempts_or_root_budget` |  |
| The application keeps its existing delivery profile | 1; `all_declared_profiles_preserve_their_uncertain_delivery_policy`<br>`default_profile_plain_retry_keeps_existing_unknown_delivery_policy` |  |
### Operator CLI preserves generic Resource semantics

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| A server returns valid JSON for another work item | 1; `control_responses_match_request_identity_operation_and_version_transition`<br>`control_interrupt_does_not_wait_on_blocked_stdout_or_stderr` |  |
| Maintenance preserves operator evidence | 1; `archive_restore_and_retention_preserve_historical_operator_receipts`<br>`current_archive_and_restore_preserve_reconciliation_hold_and_profile` | Shared snapshot archive/restore/retention tests are direct; native migration preservation also uses inspected shared snapshot state-copy/validation. Do not label this a distinct native migration seeded-operator test. |
### A release identity profile uses real provider evidence

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| An actual service token reaches an ordinary Resource | 4; Actual provider acceptance closure on both stores | Actual Node/CLI pinned-provider acceptance; unlinked/expiry/binding/rotation denial cases are in journey. Separate command ./demo/verify-provider; Executed in gate4 on37c4f87c. |
### Authentication acquisition has bounded owned execution

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| A caller leaves during blocking verification | 1, 4, 6; `caller_cancellation_retains_capacity_and_cancelled_drain_can_be_reawaited`<br>`caller_deadline_retains_capacity_until_verifier_completion` |  |
### Provider configuration cannot grant arbitrary acquisition authority

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Configuration changes while evidence is obtained | 1, 4, 6; `provider_revision_change_while_verifying_denies_stale_binding_without_holding_core_gate` |  |
### Local provisioning is explicit and restartable

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| Provisioning stops between Resource commits | 1, 4, 6; `each_lost_ack_step_resumes_after_native_close_and_reopen`<br>`both_store_resume_preserves_revisions_and_changed_input_is_rejected` |  |
### Native extension releases expose versioned conformance

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| An external adapter runs the baseline profile | 1, 5, 6; `external_fixture_passes_storage_profile_and_releases_owners_before_reopen`<br>`broken_fixture_fails_named_counts_assertion`<br>`shared_atomic_bundle_and_noop_identity_validation` | Maintained callers share the public runner; engine-specific interruption cases remain additional acceptance. |
| An author selects an obsolete profile | 1, 5, 6; `wrong_profile_fails_before_factory_operations`<br>`command rejects incompatible profile feature and missing asset before executing any example` |  |
### Author skills have portable executable evidence

| Exact scenario | Actual gates / evidence | Qualification |
| --- | --- | --- |
| A skill is extracted outside the development checkout | 5; `portable bundle has deterministic content and verifies outside its source directory` | Independent held-out author tasks executed; evaluator familiarity retained; full source-checkout prerequisite explicit. |

## Evidence limits and release status

The callback-reading-storage scenario retains its structural qualification: native query access takes no application callback, and current policies/codecs run after that read returns. No dedicated reentrant-policy test was invented. Operator metadata preservation combines direct shared snapshot archive/restore/retention tests with inspected native migration state-copy/validation; this does not claim a distinct seeded native operator migration test.

Actual process-exit tests and signal tests do not certify power-loss durability. The synthetic host process signal cases reopen previously completed attachment bytes; they do not pause active attachment work. Existing BlobService paused-publication drain and provider paused-authentication evidence remain separate component cases.

Provider service-token tests do not imply human login, production TLS, every OAuth provider or a composed provider-to-migration experiment. The skill gates and earlier agent author tasks establish runnable workflows within Linux/full-source prerequisites, not human usability or causal productivity gain. Independent Field identities/registry remain open.

The final manifest has `complete: true`, `distribution: source-only` and `publication_enabled: false`. Native format8, archive6 and profile/operator/query versions match the canonical profile. The source archive SHA-256 is `37b941ed7282e3ccdca003429614995378d8560d938db2cf454598355f733ea4`; the skills archive SHA-256 is `94b7d219c66e9e62ce4aa930bf72b790e64fe4782323da8e41ba871f935a10e6`. Extraction records four admission preflights with `examples_executed: false`; gate5 separately executed all four examples. The producer’s exclusive completed output is present, with no failed-stage output substituted.

`final-producer-scenario-execution.json` maps all 55 exact scenarios to final gate partitions and observed named cases. `final-producer-evidence-hashes.json` identifies the completed logs, manifest, checksums and archives. `audit-completed-artifact.mjs` and `final-artifact-audit.log` retain independent snapshot/log matching. The first-producer report, scenario map and evidence hashes remain historical evidence. No full gates or behavioral tests were repeated by this final evidence reviewer.
