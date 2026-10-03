#[path = "operator_runtime/support.rs"]
mod support;
use rom::operator::*;
use rom::*;
use std::sync::atomic::Ordering;
use support::*;

#[tokio::test]
async fn default_operator_policy_denies_views_and_controls_without_disclosing_existence() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let runtime = fixture.runtime(None);
        let request = request(&fixture.snapshot(), "denied");
        assert_eq!(
            runtime.operator_capabilities(&actor()).await.unwrap(),
            OperatorCapabilities::default()
        );
        assert_eq!(
            runtime.work_list(&actor(), query(1)).await,
            Err(Error::Denied)
        );
        for handle in [request.handle.clone(), WorkHandle::from_work_id("absent")] {
            assert_eq!(
                runtime.work_read(&actor(), handle).await,
                Err(Error::Denied)
            );
        }
        let before = fixture.snapshot();
        assert_eq!(
            runtime.work_control(&actor(), request).await,
            Err(Error::Denied)
        );
        assert_eq!(fixture.snapshot(), before);
    }
}
#[tokio::test]
async fn inspect_and_control_permissions_are_independent_and_views_are_redacted() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible", "hidden-one"]);
        let mut policy = Policy::all();
        policy.retry = false;
        policy.reconcile = false;
        let runtime = fixture.runtime(Some(policy));
        let page = runtime.work_list(&actor(), query(64)).await.unwrap();
        assert_eq!(page.records.len(), 1);
        let text = serde_json::to_string(&page).unwrap();
        assert!(!text.contains("secret-"));
        assert!(!text.contains("hidden-one"));
        let view = runtime
            .work_read(&actor(), page.records[0].handle.clone())
            .await
            .unwrap();
        assert_eq!(view, page.records[0]);
        let denied_request = request(&fixture.snapshot(), "inspect-only");
        assert_eq!(
            runtime.work_control(&actor(), denied_request).await,
            Err(Error::Denied)
        );
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let mut policy = Policy::all();
        policy.inspect = false;
        let runtime = fixture.runtime(Some(policy));
        assert_eq!(
            runtime.work_list(&actor(), query(1)).await,
            Err(Error::Denied)
        );
        let snapshot = fixture.snapshot();
        let record=snapshot.records.iter().find(|r|matches!(&r.pending.payload,WorkPayload::Notification{source,..}if source.key.id=="visible")).unwrap();
        let mut req = request(&snapshot, "control-only");
        req.handle = WorkHandle::from_work_id(&record.pending.id);
        req.expected = snapshot.version(record);
        assert_eq!(
            runtime.work_control(&actor(), req).await.unwrap().outcome,
            WorkControlOutcome::Scheduled
        );
    }
}
#[tokio::test]
async fn exact_control_replay_rechecks_current_operator_and_actor_authority() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let policy = Policy::all();
        let revoked = policy.revoked.clone();
        let runtime = fixture.runtime(Some(policy));
        let req = request(&fixture.snapshot(), "replay");
        let accepted = runtime.work_control(&actor(), req.clone()).await.unwrap();
        assert!(!accepted.replayed);
        let before = fixture.snapshot();
        let replay = runtime.work_control(&actor(), req.clone()).await.unwrap();
        assert!(replay.replayed);
        assert_eq!(accepted.version, replay.version);
        assert_eq!(before, fixture.snapshot());
        revoked.store(true, Ordering::SeqCst);
        assert_eq!(
            runtime.work_control(&actor(), req.clone()).await,
            Err(Error::Denied)
        );
        revoked.store(false, Ordering::SeqCst);
        runtime.revoke(&actor());
        assert_eq!(
            runtime.work_control(&actor(), req).await,
            Err(Error::Denied)
        );
        assert_eq!(before, fixture.snapshot());
    }
}
#[tokio::test]
async fn authorized_pages_use_visible_limits_and_cursors_bind_filters() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["hidden-one", "one", "two", "hidden-two", "three"]);
        let runtime = fixture.runtime(Some(Policy::all()));
        let mut q = query(1);
        let mut seen = Vec::new();
        loop {
            let page = runtime.work_list(&actor(), q.clone()).await.unwrap();
            assert_eq!(page.records.len(), 1);
            seen.push(page.records[0].handle.clone());
            match page.cursor {
                Some(cursor) => q.cursor = Some(cursor),
                None => break,
            }
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 3);
        let first = runtime.work_list(&actor(), query(1)).await.unwrap();
        let mut changed = query(1);
        changed.cursor = first.cursor;
        changed.definition = Some("other".into());
        assert_eq!(
            runtime.work_list(&actor(), changed).await,
            Err(Error::HistoryGap)
        );
        let mut unsupported = request(&fixture.snapshot(), "reconcile");
        unsupported.handle = first.records[0].handle.clone();
        unsupported.expected = first.records[0].version.clone();
        unsupported.operation = WorkControlOperation::Reconcile { evidence_ref: None };
        let before = fixture.snapshot();
        assert!(matches!(
            runtime.work_control(&actor(), unsupported).await,
            Err(Error::Unsupported(_))
        ));
        assert_eq!(before, fixture.snapshot());
    }
}
#[path = "operator_runtime/wrapper.rs"]
mod wrapper;
use std::sync::{Arc, Mutex};
use wrapper::Wrapped;
#[tokio::test]
async fn unsupported_adapter_does_not_advertise_operator_permissions() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let mut wrapped = Wrapped::new(fixture.store.clone());
        wrapped.operator_support = false;
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        assert_eq!(
            runtime.operator_capabilities(&actor()).await.unwrap(),
            OperatorCapabilities::default()
        );
        assert!(matches!(
            runtime.work_list(&actor(), query(1)).await,
            Err(Error::Unsupported(_))
        ));
    }
}
#[tokio::test]
async fn response_limits_refuse_new_controls_before_commit_and_preserve_unknown_for_committed_replays()
 {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let runtime = fixture.runtime(Some(Policy::all()));
        let original = request(&fixture.snapshot(), "accepted");
        let mut accepted = runtime
            .work_control(&actor(), original.clone())
            .await
            .unwrap();
        accepted.replayed = true;
        let bound = serde_json::to_vec(&accepted).unwrap().len() - 1;
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .operator_limits(OperatorLimits {
                responses: WorkResponseLimits {
                    max_records: 64,
                    max_bytes: bound,
                },
                ..OperatorLimits::default()
            })
            .build(fixture.store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let before = fixture.snapshot();
        let fresh = request(&before, "too-large");
        assert_eq!(
            runtime.work_control(&actor(), fresh).await,
            Err(Error::TooLarge)
        );
        assert_eq!(fixture.snapshot(), before);
        assert_eq!(
            runtime.work_control(&actor(), original).await,
            Err(Error::Unknown)
        );
        assert_eq!(fixture.snapshot(), before);
    }
}
#[tokio::test]
async fn post_commit_wrong_target_response_is_unknown_and_exact_replay_uses_saved_receipt() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let mut wrapped = Wrapped::new(fixture.store.clone());
        wrapped.control = Some(Arc::new(|mut receipt| {
            receipt.result.handle = WorkHandle::from_work_id("wrong-target");
            Ok(receipt)
        }));
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let original = request(&fixture.snapshot(), "unknown");
        assert_eq!(
            runtime.work_control(&actor(), original.clone()).await,
            Err(Error::Unknown)
        );
        assert_eq!(fixture.snapshot().operator.receipts.len(), 1);
        assert!(
            runtime
                .work_control(&actor(), original)
                .await
                .unwrap()
                .replayed
        );
    }
}
#[tokio::test]
async fn committed_retry_completed_is_unknown_and_exact_replay_recovers() {
    incompatible_committed_outcome(WorkControlOutcome::Completed).await;
}
#[tokio::test]
async fn committed_retry_stopped_is_unknown_and_exact_replay_recovers() {
    incompatible_committed_outcome(WorkControlOutcome::Stopped(StopReason::Attempts)).await;
}
#[tokio::test]
async fn committed_retry_unresolved_is_unknown_and_exact_replay_recovers() {
    incompatible_committed_outcome(WorkControlOutcome::Unresolved).await;
}
async fn incompatible_committed_outcome(outcome: WorkControlOutcome) {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let mut wrapped = Wrapped::new(fixture.store.clone());
        let outcome = outcome.clone();
        wrapped.control = Some(Arc::new(move |mut receipt| {
            receipt.result.outcome = outcome.clone();
            Ok(receipt)
        }));
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let original = request(&fixture.snapshot(), "malformed-outcome");
        assert_eq!(
            runtime.work_control(&actor(), original.clone()).await,
            Err(Error::Unknown)
        );
        let committed = fixture.snapshot();
        assert_eq!(committed.operator.receipts.len(), 1);
        let result = runtime.work_control(&actor(), original).await.unwrap();
        assert!(result.replayed);
        assert_eq!(result.outcome, WorkControlOutcome::Scheduled);
        assert_eq!(fixture.snapshot(), committed);
    }
}
#[tokio::test]
async fn moving_page_continues_after_anchor_is_absent_from_current_snapshot() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["one", "two", "three"]);
        let hidden = Arc::new(Mutex::new(None::<WorkHandle>));
        let state = hidden.clone();
        let mut wrapped = Wrapped::new(fixture.store.clone());
        wrapped.snapshot = Some(Arc::new(move |mut snapshot| {
            if let Some(handle) = &*state.lock().unwrap() {
                snapshot
                    .records
                    .retain(|record| WorkHandle::from_work_id(&record.pending.id) != *handle);
            }
            Ok(snapshot)
        }));
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let first = runtime.work_list(&actor(), query(1)).await.unwrap();
        *hidden.lock().unwrap() = Some(first.records[0].handle.clone());
        let mut next = query(1);
        next.cursor = first.cursor;
        let second = runtime.work_list(&actor(), next).await.unwrap();
        assert_eq!(second.records.len(), 1);
        assert!(second.records[0].handle > first.records[0].handle);
    }
}
#[path = "operator_runtime/resolution.rs"]
mod resolution;
#[tokio::test]
async fn moving_page_rechecks_scope_permission_after_anchor_revocation() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["one", "two", "three"]);
        let denied = Arc::new(Mutex::new(None::<String>));
        let policy = denied.clone();
        let runtime = builder()
            .operator_authorizer(Arc::new(
                move |_: &Actor,
                      _: OperatorAccess,
                      scope: Option<&WorkScope>,
                      _: &mut dyn AuthorizationRead| {
                    if scope.is_some_and(|scope| {
                        scope
                            .source
                            .as_ref()
                            .is_some_and(|key| policy.lock().unwrap().as_ref() == Some(&key.id))
                    }) {
                        Err(Error::Denied)
                    } else {
                        Ok(())
                    }
                },
            ))
            .build(fixture.store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let first = runtime.work_list(&actor(), query(1)).await.unwrap();
        *denied.lock().unwrap() = Some(first.records[0].source.as_ref().unwrap().id.clone());
        let mut next = query(1);
        next.cursor = first.cursor;
        let page = runtime.work_list(&actor(), next).await.unwrap();
        assert_eq!(page.records.len(), 1);
        assert!(page.records[0].handle > first.records[0].handle);
    }
}
#[tokio::test]
async fn cancelled_control_keeps_supervision_and_capacity_until_accepted_commit_finishes() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let (entered, seen) = std::sync::mpsc::sync_channel(1);
        let (release, wait) = std::sync::mpsc::sync_channel(1);
        let wait = Mutex::new(wait);
        let mut wrapped = Wrapped::new(fixture.store.clone());
        wrapped.control = Some(Arc::new(move |receipt| {
            entered.send(()).unwrap();
            wait.lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap();
            Ok(receipt)
        }));
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .limits(Limits {
                io_jobs: 1,
                ..Limits::default()
            })
            .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let original = request(&fixture.snapshot(), "cancelled");
        let clone = runtime.clone();
        let req = original.clone();
        let caller = tokio::spawn(async move { clone.work_control(&actor(), req).await });
        tokio::task::spawn_blocking(move || {
            seen.recv_timeout(std::time::Duration::from_secs(3))
                .unwrap()
        })
        .await
        .unwrap();
        caller.abort();
        let _ = caller.await;
        assert_eq!(runtime.available_io_capacity(), 0);
        assert_eq!(runtime.status().unwrap().owned_work, 1);
        assert_eq!(fixture.snapshot().operator.receipts.len(), 1);
        assert_eq!(
            runtime.work_list(&actor(), query(1)).await,
            Err(Error::Overloaded)
        );
        release.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), runtime.shutdown())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(runtime.status().unwrap().owned_work, 0);
        drop(runtime);
        let runtime = fixture.runtime(Some(Policy::all()));
        assert!(
            runtime
                .work_control(&actor(), original)
                .await
                .unwrap()
                .replayed
        );
        assert_eq!(fixture.snapshot().operator.receipts.len(), 1);
    }
}
struct Gate {
    revoked: std::sync::atomic::AtomicBool,
}
impl ActorGate for Gate {
    fn check(&self, _: &Actor, _: &mut dyn AuthorizationRead) -> Result<()> {
        if self.revoked.load(Ordering::SeqCst) {
            Err(Error::Invalid {
                kind: "secret-gate-body".into(),
                field: "secret-credential".into(),
            })
        } else {
            Ok(())
        }
    }
}
#[tokio::test]
async fn authoritative_gate_revocation_denies_saved_receipts_without_callback_text() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let gate = Arc::new(Gate {
            revoked: std::sync::atomic::AtomicBool::new(false),
        });
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .actor_gate(gate.clone())
            .build(fixture.store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let req = request(&fixture.snapshot(), "gate-replay");
        runtime.work_control(&actor(), req.clone()).await.unwrap();
        let before = fixture.snapshot();
        gate.revoked.store(true, Ordering::SeqCst);
        assert_eq!(
            runtime.work_control(&actor(), req).await,
            Err(Error::Denied)
        );
        assert_eq!(
            runtime.operator_capabilities(&actor()).await,
            Err(Error::Denied)
        );
        assert_eq!(before, fixture.snapshot());
    }
}
#[tokio::test]
async fn raw_snapshot_bounds_and_generation_are_checked_before_projection() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["one", "two"]);
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .operator_limits(OperatorLimits {
                max_snapshot_records: 1,
                ..OperatorLimits::default()
            })
            .build(fixture.store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        assert_eq!(
            runtime.work_list(&actor(), query(1)).await,
            Err(Error::TooLarge)
        );
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let changed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let state = changed.clone();
        let mut wrapped = Wrapped::new(fixture.store.clone());
        wrapped.snapshot = Some(Arc::new(move |mut snapshot| {
            if state.load(Ordering::SeqCst) {
                snapshot.generation = "restored-generation".into();
            }
            Ok(snapshot)
        }));
        let runtime = builder()
            .operator_authorizer(Arc::new(Policy::all()))
            .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let first = runtime.work_list(&actor(), query(1)).await.unwrap();
        changed.store(true, Ordering::SeqCst);
        let mut next = query(1);
        next.cursor = first.cursor;
        assert_eq!(
            runtime.work_list(&actor(), next).await,
            Err(Error::HistoryGap)
        );
    }
}
#[tokio::test]
async fn accepted_receipt_remains_replayable_after_target_retirement_with_current_scope_authority()
{
    for redb in [false, true] {
        let fixture = Fixture::new(redb, &["visible"]);
        let retired = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let denied = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let state = retired.clone();
        let policy = denied.clone();
        let mut wrapped = Wrapped::new(fixture.store.clone());
        wrapped.snapshot = Some(Arc::new(move |mut snapshot| {
            if state.load(Ordering::SeqCst) {
                snapshot.records.clear();
            }
            Ok(snapshot)
        }));
        let runtime = builder()
            .operator_authorizer(Arc::new(
                move |_: &Actor,
                      _: OperatorAccess,
                      scope: Option<&WorkScope>,
                      _: &mut dyn AuthorizationRead| {
                    if scope.is_some() && policy.load(Ordering::SeqCst) {
                        Err(Error::Denied)
                    } else {
                        Ok(())
                    }
                },
            ))
            .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let original = request(&fixture.snapshot(), "retired");
        runtime
            .work_control(&actor(), original.clone())
            .await
            .unwrap();
        retired.store(true, Ordering::SeqCst);
        assert!(
            runtime
                .work_control(&actor(), original.clone())
                .await
                .unwrap()
                .replayed
        );
        denied.store(true, Ordering::SeqCst);
        assert_eq!(
            runtime.work_control(&actor(), original).await,
            Err(Error::Denied)
        );
    }
}
#[tokio::test]
async fn concurrent_runtime_controls_arbitrate_stale_views_and_exact_identities() {
    for redb in [false, true] {
        for same in [false, true] {
            let fixture = Fixture::new(redb, &["visible"]);
            let runtime = fixture.runtime(Some(Policy::all()));
            let first = request(&fixture.snapshot(), "first");
            let mut second = first.clone();
            if !same {
                second.key = "second".into();
            }
            let operator = actor();
            let (left, right) = tokio::join!(
                runtime.work_control(&operator, first),
                runtime.work_control(&operator, second)
            );
            if same {
                assert!(left.is_ok() && right.is_ok());
                assert_ne!(left.unwrap().replayed, right.unwrap().replayed);
            } else {
                assert!(
                    (left.is_ok() && right == Err(Error::Conflict))
                        || (right.is_ok() && left == Err(Error::Conflict))
                );
            }
            assert_eq!(fixture.snapshot().operator.receipts.len(), 1);
        }
    }
}
