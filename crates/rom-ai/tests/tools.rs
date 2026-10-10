//! Typed public tool declarations; database journeys are added before runtime implementation.
use rom_ai::PreparedToolAction;
use rom_ai::{AiError, ReadContext, ToolRegistry};

#[path = "tools/fault.rs"]
mod fault;
#[path = "work_flows/fault.rs"]
mod settlement_fault;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_admission_noncommit_reopens_original_identity() {
    support::recovery(false, 25).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_admission_noncommit_reopens_original_identity() {
    support::recovery(true, 25).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_admission_lost_ack_reopens_original_identity() {
    support::recovery(false, 26).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_admission_lost_ack_reopens_original_identity() {
    support::recovery(true, 26).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_frozen_retry_budget_exhaustion_is_actionable() {
    support::recovery(false, 24).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_frozen_retry_budget_exhaustion_is_actionable() {
    support::recovery(true, 24).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_unstarted_reopened_call_fences_original_delivery() {
    support::recovery(false, 23).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_unstarted_reopened_call_fences_original_delivery() {
    support::recovery(true, 23).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_different_owner_changes_no_run_account_or_callback() {
    support::recovery(false, 22).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_different_owner_changes_no_run_account_or_callback() {
    support::recovery(true, 22).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_expiry_reports_failed_without_read_or_charge() {
    support::recovery(false, 21).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_expiry_reports_failed_without_read_or_charge() {
    support::recovery(true, 21).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_revoked_grant_reports_failed_without_read_or_charge() {
    support::recovery(false, 20).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_revoked_grant_reports_failed_without_read_or_charge() {
    support::recovery(true, 20).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_coalesces_duplicates_and_replays_current_projection() {
    support::recovery(false, 19).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_coalesces_duplicates_and_replays_current_projection() {
    support::recovery(true, 19).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_read_resume_rejects_live_cpu_after_callback_timeout_then_retries_once() {
    support::recovery(false, 18).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_read_resume_rejects_live_cpu_after_callback_timeout_then_retries_once() {
    support::recovery(true, 18).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_reopens_same_call_without_restarting_generation() {
    support::recovery(false, 17).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_reopens_same_call_without_restarting_generation() {
    support::recovery(true, 17).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_temporary_or_unknown_read_error_keeps_original_tool_pending() {
    for mode in [12, 13, 14, 15, 16] {
        support::recovery(false, mode).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_temporary_or_unknown_read_error_keeps_original_tool_pending() {
    for mode in [12, 13, 14, 15, 16] {
        support::recovery(true, mode).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_definite_read_tool_failure_reports_actionable_failure_without_more_io() {
    support::recovery(false, 11).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_definite_read_tool_failure_reports_actionable_failure_without_more_io() {
    support::recovery(true, 11).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_revoked_paid_grant_blocks_tool_settlement_recovery_without_new_charge() {
    support::paid_revoked_grant(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_revoked_paid_grant_blocks_tool_settlement_recovery_without_new_charge() {
    support::paid_revoked_grant(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_paid_tool_settlement_noncommit_recovers_before_any_tool_io() {
    support::paid_settlement(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_paid_tool_settlement_noncommit_recovers_before_any_tool_io() {
    support::paid_settlement(true, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_paid_tool_settlement_lost_ack_replays_exact_original_cost() {
    support::paid_settlement(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_paid_tool_settlement_lost_ack_replays_exact_original_cost() {
    support::paid_settlement(true, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_grant_revoked_during_calculation_blocks_private_result_publication() {
    support::recovery(false, 10).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_grant_revoked_during_calculation_blocks_private_result_publication() {
    support::recovery(true, 10).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_repeated_native_call_id_holds_valid_provider_knowledge_without_more_effects() {
    support::recovery(false, 9).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_repeated_native_call_id_holds_valid_provider_knowledge_without_more_effects() {
    support::recovery(true, 9).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_typed_null_result_remains_completed_through_durable_native_continuation() {
    support::recovery(false, 8).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_typed_null_result_remains_completed_through_durable_native_continuation() {
    support::recovery(true, 8).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_concurrent_and_duplicate_tool_resume_admits_one_original_receipt_replay() {
    support::recovery(false, 4).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_concurrent_and_duplicate_tool_resume_admits_one_original_receipt_replay() {
    support::recovery(true, 4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_source_row_grant_revocation_denies_retained_private_output() {
    support::recovery(false, 7).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_source_row_grant_revocation_denies_retained_private_output() {
    support::recovery(true, 7).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_revoked_tool_grant_blocks_successor_generation_and_reports_failure() {
    support::recovery(false, 5).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_revoked_tool_grant_blocks_successor_generation_and_reports_failure() {
    support::recovery(true, 5).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_revoked_tool_grant_blocks_retained_output_disclosure() {
    support::recovery(false, 6).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_revoked_tool_grant_blocks_retained_output_disclosure() {
    support::recovery(true, 6).await;
}
#[path = "tools/support.rs"]
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_unknown_tool_action_replays_original_receipt_after_restart() {
    support::recovery(false, 0).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_unknown_tool_action_replays_original_receipt_after_restart() {
    support::recovery(true, 0).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_current_tool_grant_revocation_blocks_recovery_without_provider_lookup() {
    support::recovery(false, 1).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_current_tool_grant_revocation_blocks_recovery_without_provider_lookup() {
    support::recovery(true, 1).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_cancel_pending_tool_after_lost_ack_keeps_uncertainty() {
    support::recovery(false, 2).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_cancel_pending_tool_after_lost_ack_keeps_uncertainty() {
    support::recovery(true, 2).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_cancel_tools_before_invocation_prevents_side_effects() {
    support::recovery(false, 3).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_cancel_tools_before_invocation_prevents_side_effects() {
    support::recovery(true, 3).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_typed_tools_continue_original_request_and_commit_once() {
    support::journey(false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_typed_tools_continue_original_request_and_commit_once() {
    support::journey(true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_historical_tool_actor_source_and_global_call_id_forgery_changes_no_state_or_events()
{
    support::journey(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_historical_tool_actor_source_and_global_call_id_forgery_changes_no_state_or_events() {
    support::journey(true, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_submission_freezes_registry_and_replays_original_version_after_restart() {
    support::registry_freeze(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_submission_freezes_registry_and_replays_original_version_after_restart() {
    support::registry_freeze(true).await;
}

#[test]
fn decoded_action_contract_rejects_builtin_mutations_and_unbounded_identity() {
    let prepared = serde_json::json!({
        "registry_version":1,"tool_name":"classify_task","resource_version":1,
        "call_id":"write-1","actor":{"authority":"tool-tests","subject":"tool-owner","kind":"human"},
        "invocation":{"kind":"typed_tool_task","id":"task","expected":1,"idempotency":"ai-tool-run-write-1","operation":{"type":"delete"}}
    });
    assert!(
        serde_json::from_value::<PreparedToolAction>(prepared)
            .unwrap()
            .validate()
            .is_err()
    );
    let forged = serde_json::json!({
        "registry_version":1,"tool_name":"classify_task","resource_version":1,
        "call_id":"write-1","actor":{"authority":"x".repeat(20000),"subject":"tool-owner","kind":"human"},
        "invocation":{"kind":"typed_tool_task","id":"task","expected":1,"idempotency":"ai-tool-run-write-1","operation":{"type":"action","input":{"name":"classify","input":"reviewed"}}}
    });
    assert!(
        serde_json::from_value::<PreparedToolAction>(forged)
            .unwrap()
            .validate()
            .is_err()
    );
}

#[test]
fn registry_rejects_unversioned_duplicate_and_unbounded_declarations() {
    assert!(matches!(ToolRegistry::new(0), Err(AiError::InvalidRequest)));
    let mut registry = ToolRegistry::new(1).unwrap();
    registry
        .read::<String, String, _>(
            "echo",
            "Read an authorized value",
            |_: ReadContext, input| Box::pin(async move { Ok(input) }),
        )
        .unwrap();
    assert!(
        registry
            .read::<String, String, _>("echo", "Duplicate", |_, input| Box::pin(async move {
                Ok(input)
            }))
            .is_err()
    );
    assert!(
        registry
            .read::<String, String, _>("forged name", "Invalid", |_, input| Box::pin(async move {
                Ok(input)
            }))
            .is_err()
    );
    assert!(
        registry
            .read::<String, String, _>("oversized", &"x".repeat(2049), |_, input| Box::pin(
                async move { Ok(input) }
            ))
            .is_err()
    );
    for index in 1..16 {
        registry
            .read::<String, String, _>(&format!("read_{index}"), "Read", |_, input| {
                Box::pin(async move { Ok(input) })
            })
            .unwrap();
    }
    assert!(
        registry
            .read::<String, String, _>("seventeenth", "Read", |_, input| Box::pin(async move {
                Ok(input)
            }))
            .is_err()
    );
    assert_eq!(registry.descriptors().len(), 16);
}

#[test]
fn native_read_schema_uses_typed_input_and_rejects_model_selected_authority() {
    let mut registry = ToolRegistry::new(4).unwrap();
    registry
        .read::<String, String, _>("private_read", "Host-selected description", |_, input| {
            Box::pin(async move { Ok(input) })
        })
        .unwrap();
    let descriptors = registry.descriptors();
    assert_eq!(descriptors[0].name(), "private_read");
    let schema = descriptors[0].parameters();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["input"]["type"], "string");
    assert_eq!(schema["required"], serde_json::json!(["input"]));
    assert_eq!(schema["properties"].as_object().unwrap().len(), 1);
    assert!(!format!("{registry:?}").contains("Host-selected description"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_resume_future_size_characterization() {
    support::read_future_size().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_claimed_unknown_before_callback_preserves_ordinal() {
    support::recovery(false, 27).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_claimed_unknown_before_callback_preserves_ordinal() {
    support::recovery(true, 27).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_result_noncommit_reopens_with_exact_original_attempt() {
    support::recovery(false, 28).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_result_lost_ack_reopens_with_exact_original_attempt() {
    support::recovery(false, 29).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_result_noncommit_reopens_with_exact_original_attempt() {
    support::recovery(true, 28).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_result_lost_ack_reopens_with_exact_original_attempt() {
    support::recovery(true, 29).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_resume_authority_wait_consumes_shared_deadline_without_admission() {
    support::recovery(false, 30).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_resume_authority_wait_consumes_shared_deadline_without_admission() {
    support::recovery(true, 30).await;
}

#[test]
fn public_read_progress_contract_exposes_only_finite_status_and_ordinal() {
    use rom_ai::flow::{ReadProgress, ReadStatus, RunView};
    fn public_progress(view: &RunView) -> Option<(ReadStatus, u32)> {
        view.read_progress()
            .map(|progress| (progress.status(), progress.ordinal()))
    }
    let _: fn(&RunView) -> Option<(ReadStatus, u32)> = public_progress;
    let _: Option<ReadProgress> = None;
    let wire = serde_json::to_value([
        ReadStatus::Queued,
        ReadStatus::Active,
        ReadStatus::AwaitingRecovery,
        ReadStatus::ActivityUnknown,
    ])
    .unwrap();
    assert_eq!(
        wire,
        serde_json::json!(["Queued", "Active", "AwaitingRecovery", "ActivityUnknown"])
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_progress_shows_advisory_recovery_and_queued_snapshots() {
    support::recovery(false, 31).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_read_progress_rejects_current_referenced_row_grant_revocation() {
    support::recovery(false, 32).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_progress_shows_advisory_recovery_and_queued_snapshots() {
    support::recovery(true, 31).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_read_progress_rejects_current_referenced_row_grant_revocation() {
    support::recovery(true, 32).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_and_redb_physical_read_recovery_preserve_ownership_concurrently() {
    // Keep the two large recovery states on the heap while polling both concurrently.
    let sqlite = Box::pin(support::recovery(false, 18));
    let redb = Box::pin(support::recovery(true, 18));
    eprintln!(
        "concurrent recovery child state bytes: SQLite={}, Redb={}",
        std::mem::size_of_val(sqlite.as_ref().get_ref()),
        std::mem::size_of_val(redb.as_ref().get_ref())
    );
    let concurrent = async move {
        tokio::join!(sqlite, redb);
    };
    let bytes = std::mem::size_of_val(&concurrent);
    eprintln!("boxed concurrent recovery future bytes: {bytes}");
    assert!(
        bytes <= 1024,
        "concurrent fixture must retain handles rather than inline recovery states"
    );
    concurrent.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_pending_duplicate_resume_cannot_finish_operation_before_original_tool_recovery() {
    support::deterministic_resume_race(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_pending_duplicate_resume_cannot_finish_operation_before_original_tool_recovery() {
    support::deterministic_resume_race(true).await;
}
