use crate::{fixture::*, model::*};
use rom::{Command, DeliveryOutcome, Resource, StopReason, WorkPayload, WorkState};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::atomic::Ordering};

pub async fn stock_setup(f: &Fixture, trace: &mut Vec<Value>, enabled: bool) -> Inventory {
    let initial = Inventory {
        total: 10,
        reservations: BTreeMap::new(),
        released: vec![],
    };
    f.create("sku", initial.clone(), trace).await;
    f.create("flow-a", Workflow::new("stock", "sku", enabled), trace)
        .await;

    f.act(
        "sku",
        RESERVE,
        Reserve {
            token: "flow-a".into(),
            quantity: 3,
        },
        "reserve-a",
        trace,
    )
    .await;
    f.act(
        "sku",
        RESERVE,
        Reserve {
            token: "flow-b".into(),
            quantity: 2,
        },
        "independent-reserve-b",
        trace,
    )
    .await;
    f.act("sku", RESTOCK, 5, "independent-restock", trace).await;
    f.drain(trace).await;
    initial
}
async fn fail(f: &Fixture, trace: &mut Vec<Value>) {
    f.act(
        "flow-a",
        OBSERVE,
        "confirmed_permanent".into(),
        "confirmed-rejection",
        trace,
    )
    .await;
}
fn stock_compensated(stock: &Inventory) {
    assert_eq!(stock.total, 15);
    assert_eq!(stock.reservations, BTreeMap::from([("flow-b".into(), 2)]));
    assert_eq!(stock.released, ["flow-a"]);
    assert_eq!(stock.available(), 13);
}
async fn compare_stock(backend: &str, strategy: &str) -> Value {
    let dir = Scratch::new();
    let f = Fixture::open(backend, &dir.0.join("db"), Options::default());
    let mut trace = vec![];
    let initial = stock_setup(&f, &mut trace, strategy == "targeted").await;
    fail(&f, &mut trace).await;
    f.drain(&mut trace).await;
    if strategy == "stale_snapshot" {
        let current = f.read::<Inventory>("sku").await;
        let unsafe_restore = f
            .runtime
            .execute(
                &author(),
                Command::replace("sku", initial)
                    .at_revision(current.revision)
                    .idempotency("deliberately-wrong-restore"),
            )
            .await
            .unwrap();
        record(
            "unsafe_old_content_with_valid_current_revision",
            &unsafe_restore,
            &mut trace,
        );
    }
    let row = f.read::<Inventory>("sku").await;
    record("final_inventory", &row, &mut trace);
    let stock = row.value.unwrap();
    match strategy {
        "targeted" => stock_compensated(&stock),
        "none" => {
            assert_eq!(stock.reservations.len(), 2);
            assert_eq!(stock.total, 15);
        }
        "stale_snapshot" => {
            assert!(stock.reservations.is_empty());
            assert_eq!(stock.total, 10);
        }
        _ => unreachable!(),
    }
    let result = evidence(
        backend,
        &format!("stock_{strategy}"),
        if strategy == "targeted" {
            "targeted_compensation_preserves_other_work"
        } else {
            "expected_counterexample"
        },
        trace,
        json!({"available":stock.available(),"other_reservation_preserved":stock.reservations.contains_key("flow-b"),"restock_preserved":stock.total==15,"own_reservation_released":!stock.reservations.contains_key("flow-a")}),
    );
    finish_result(f, result).await
}
async fn compare_seat(backend: &str, strategy: &str) -> Value {
    let dir = Scratch::new();
    let f = Fixture::open(backend, &dir.0.join("db"), Options::default());
    let mut trace = vec![];
    let initial = Seating {
        seats: BTreeMap::new(),
        cancelled: vec![],
    };
    f.create("show", initial.clone(), &mut trace).await;
    f.create(
        "flow-a",
        Workflow::new("seat", "show", strategy == "targeted"),
        &mut trace,
    )
    .await;

    f.act(
        "show",
        BOOK,
        BTreeMap::from([("A1".into(), "flow-a".into())]),
        "book-a",
        &mut trace,
    )
    .await;
    f.act(
        "show",
        BOOK,
        BTreeMap::from([("A2".into(), "flow-b".into())]),
        "independent-book-b",
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    fail(&f, &mut trace).await;
    f.drain(&mut trace).await;
    if strategy == "stale_snapshot" {
        let now = f.read::<Seating>("show").await;
        let restored = f
            .runtime
            .execute(
                &author(),
                Command::replace("show", initial)
                    .at_revision(now.revision)
                    .idempotency("bad-seat-restore"),
            )
            .await
            .unwrap();
        record(
            "unsafe_old_content_with_valid_current_revision",
            &restored,
            &mut trace,
        );
    }
    let row = f.read::<Seating>("show").await;
    record("final_seating", &row, &mut trace);
    let seats = row.value.unwrap();
    match strategy {
        "targeted" => {
            assert_eq!(
                seats.seats,
                BTreeMap::from([("A2".into(), "flow-b".into())])
            );
            assert_eq!(seats.cancelled, ["flow-a"]);
        }
        "none" => assert_eq!(seats.seats.len(), 2),
        "stale_snapshot" => assert!(seats.seats.is_empty()),
        _ => unreachable!(),
    }
    let result = evidence(
        backend,
        &format!("seat_{strategy}"),
        if strategy == "targeted" {
            "cancel_only_owned_booking"
        } else {
            "expected_counterexample"
        },
        trace,
        json!({"other_booking_preserved":seats.seats.contains_key("A2"),"own_booking_released":!seats.seats.contains_key("A1")}),
    );
    finish_result(f, result).await
}
async fn compare_account(backend: &str, strategy: &str) -> Value {
    let dir = Scratch::new();
    let f = Fixture::open(backend, &dir.0.join("db"), Options::default());
    let mut trace = vec![];
    let initial = Account {
        owner: "flow-a".into(),
        enabled: false,
        note: "".into(),
        audit: vec!["requested".into()],
    };
    f.create("alice", initial.clone(), &mut trace).await;
    f.create(
        "flow-a",
        Workflow::new("account", "alice", strategy == "targeted"),
        &mut trace,
    )
    .await;

    f.act("alice", PROVISION, (), "provision-alice", &mut trace)
        .await;
    f.act(
        "alice",
        ANNOTATE,
        "support case retained".into(),
        "independent-support-edit",
        &mut trace,
    )
    .await;
    f.create(
        "bob",
        Account {
            owner: "flow-b".into(),
            enabled: true,
            note: "independent account".into(),
            audit: vec!["requested".into(), "provisioned".into()],
        },
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    fail(&f, &mut trace).await;
    f.drain(&mut trace).await;
    if strategy == "stale_snapshot" {
        let now = f.read::<Account>("alice").await;
        let restored = f
            .runtime
            .execute(
                &author(),
                Command::replace("alice", initial)
                    .at_revision(now.revision)
                    .idempotency("bad-account-restore"),
            )
            .await
            .unwrap();
        record(
            "unsafe_old_content_with_valid_current_revision",
            &restored,
            &mut trace,
        );
    }
    let alice = f.read::<Account>("alice").await;
    let bob = f.read::<Account>("bob").await;
    record("final_alice", &alice, &mut trace);
    record("final_bob", &bob, &mut trace);
    let alice = alice.value.unwrap();
    assert!(bob.value.unwrap().enabled);
    match strategy {
        "targeted" => {
            assert!(!alice.enabled);
            assert_eq!(alice.note, "support case retained");
            assert_eq!(
                alice.audit,
                [
                    "requested",
                    "provisioned",
                    "independent_annotation",
                    "compensating_disable"
                ]
            );
        }
        "none" => assert!(alice.enabled),
        "stale_snapshot" => {
            assert_eq!(alice.audit, ["requested"]);
            assert!(alice.note.is_empty());
        }
        _ => unreachable!(),
    }
    let history = f
        .runtime
        .journal(&author(), Account::KIND, None)
        .await
        .unwrap();
    assert!(history.events.len() >= 4);
    let result = evidence(
        backend,
        &format!("account_{strategy}"),
        if strategy == "targeted" {
            "disable_specific_account_without_erasing_current_audit"
        } else {
            "expected_counterexample"
        },
        trace,
        json!({"enabled":alice.enabled,"current_audit_entries":alice.audit.len(),"other_account_enabled":true,"rom_journal_facts_remain":history.events.len()}),
    );
    finish_result(f, result).await
}
async fn unresolved(backend: &str, unknown: bool) -> Value {
    let dir = Scratch::new();
    let options = Options::default();
    options.provider.lock().unwrap().script.extend([
        if unknown {
            DeliveryOutcome::Unknown
        } else {
            DeliveryOutcome::Retryable
        },
        DeliveryOutcome::Accepted,
    ]);
    let f = Fixture::open(backend, &dir.0.join("db"), options);
    let mut trace = vec![];
    stock_setup(&f, &mut trace, true).await;
    f.act(
        "flow-a",
        DISPATCH,
        "flow-a".into(),
        "provider-request",
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    f.act(
        "flow-a",
        OBSERVE,
        if unknown { "unknown" } else { "transient" }.into(),
        "record-unresolved-outcome",
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    let held = f.read::<Inventory>("sku").await;
    record("hold_while_unresolved", &held, &mut trace);
    assert!(held.value.unwrap().reservations.contains_key("flow-a"));
    let first_calls = f.options.provider.lock().unwrap().deliveries.clone();
    assert_eq!(first_calls.len(), 1);
    trace.push(json!({"step":"simulated_provider_before_reconciliation","calls":first_calls,"provider_effect_may_exist":unknown}));
    f.options.clock.advance();
    f.drain(&mut trace).await;
    f.act(
        "flow-a",
        OBSERVE,
        "succeeded".into(),
        "explicit-simulated-success-confirmation",
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    let stock = f.read::<Inventory>("sku").await;
    record("after_success_no_compensation", &stock, &mut trace);
    assert!(stock.value.unwrap().reservations.contains_key("flow-a"));
    let calls = {
        let state = f.options.provider.lock().unwrap();
        assert_eq!(state.deliveries.len(), 2);
        assert_eq!(state.deliveries[0]["id"], state.deliveries[1]["id"]);
        assert_eq!(state.effects.len(), 1);
        state.deliveries.clone()
    };
    trace.push(json!({"step":"simulated_receiver_idempotency_reconciles_retry","calls":calls}));
    let result = evidence(
        backend,
        if unknown {
            "unknown_holds_then_reconciles"
        } else {
            "transient_retries_without_compensation"
        },
        "no_inferred_inverse_from_unresolved_failure",
        trace,
        json!({"compensations":0,"simulated_external_effects":1,"forward_delivery_attempts":2,"same_delivery_identity":true}),
    );
    finish_result(f, result).await
}
async fn compensation_retry(backend: &str, exhaust: bool) -> Value {
    let dir = Scratch::new();
    let f = Fixture::open(backend, &dir.0.join("db"), Options::default());
    let mut trace = vec![];
    stock_setup(&f, &mut trace, true).await;
    fail(&f, &mut trace).await;
    f.options
        .failures
        .store(if exhaust { 3 } else { 1 }, Ordering::SeqCst);
    f.drain(&mut trace).await;
    assert!(
        f.read::<Inventory>("sku")
            .await
            .value
            .unwrap()
            .reservations
            .contains_key("flow-a")
    );
    for _ in 0..3 {
        f.options.clock.advance();
        f.drain(&mut trace).await;
    }
    if exhaust {
        assert!(f.stopped(StopReason::Attempts));
        let held = f.read::<Inventory>("sku").await;
        record("terminal_manual_intervention_required", &held, &mut trace);
        assert!(held.value.unwrap().reservations.contains_key("flow-a"));
        f.act(
            "sku",
            RELEASE,
            "flow-a".into(),
            "explicit-manual-compensation",
            &mut trace,
        )
        .await;
        assert!(f.stopped(StopReason::Attempts));
    }
    let stock = f.read::<Inventory>("sku").await;
    record("final_inventory", &stock, &mut trace);
    stock_compensated(stock.value.as_ref().unwrap());
    let records = f.store.reaction_records().unwrap();
    let action = records
        .iter()
        .find(|w| {
            w.pending.definition == "stock-compensation"
                && matches!(w.pending.payload, WorkPayload::Action(_))
        })
        .unwrap();
    assert_eq!(action.attempts, if exhaust { 3 } else { 2 });
    let result = evidence(
        backend,
        if exhaust {
            "compensation_attempt_budget_then_manual"
        } else {
            "compensation_transient_failure_retries"
        },
        "compensation_is_fallible_bounded_work",
        trace,
        json!({"automatic_attempts":action.attempts,"manual_action_needed":exhaust,"target_release_entries":1}),
    );
    finish_result(f, result).await
}
async fn revoked(backend: &str) -> Value {
    let dir = Scratch::new();
    let f = Fixture::open(backend, &dir.0.join("db"), Options::default());
    let mut trace = vec![];
    stock_setup(&f, &mut trace, true).await;
    f.runtime.revoke(&compensator());
    fail(&f, &mut trace).await;
    f.drain(&mut trace).await;
    assert!(f.stopped(StopReason::Denied));
    let row = f.read::<Inventory>("sku").await;
    record("revocation_preserves_original_commit", &row, &mut trace);
    assert!(row.value.unwrap().reservations.contains_key("flow-a"));
    let result = evidence(
        backend,
        "revoked_compensator_blocks",
        "no_authority_bypass_for_cleanup",
        trace,
        json!({"compensations":0,"stopped":"Denied","manual_intervention_required":true}),
    );
    finish_result(f, result).await
}
async fn conflict(backend: &str) -> Value {
    let dir = Scratch::new();
    let f = Fixture::open(backend, &dir.0.join("db"), Options::default());
    let mut trace = vec![];
    stock_setup(&f, &mut trace, true).await;
    fail(&f, &mut trace).await;
    for _ in 0..16 {
        f.runtime.process_work(1).await.unwrap();
        if f.store.reaction_records().unwrap().iter().any(|w| {
            w.state == WorkState::Pending
                && w.pending.definition == "stock-compensation"
                && matches!(w.pending.payload, WorkPayload::Action(_))
        }) {
            break;
        }
    }
    assert!(
        f.store
            .reaction_records()
            .unwrap()
            .iter()
            .any(|w| w.state == WorkState::Pending
                && matches!(w.pending.payload, WorkPayload::Action(_)))
    );
    f.act(
        "sku",
        RESTOCK,
        7,
        "edit-after-compensation-materialization",
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    assert!(f.stopped(StopReason::Conflict));
    let row = f.read::<Inventory>("sku").await;
    record("conflict_requires_reconciliation", &row, &mut trace);
    let stock = row.value.unwrap();
    assert_eq!(stock.total, 22);
    assert_eq!(stock.reservations.len(), 2);
    let result = evidence(
        backend,
        "concurrent_target_edit_stops_stale_compensation",
        "revision_conflict_preserves_new_state",
        trace,
        json!({"total":22,"reservations_preserved":2,"stopped":"Conflict","automatic_rebase":false}),
    );
    finish_result(f, result).await
}
async fn reopen(backend: &str) -> Value {
    let dir = Scratch::new();
    let path = dir.0.join("db");
    let options = Options::default();
    let mut trace = vec![];
    let f = Fixture::open(backend, &path, options.clone());
    stock_setup(&f, &mut trace, true).await;
    fail(&f, &mut trace).await;
    f.finish().await;
    let f = Fixture::open(backend, &path, options);
    f.drain(&mut trace).await;
    let before = f.read::<Inventory>("sku").await;
    stock_compensated(before.value.as_ref().unwrap());
    record("after_clean_reopen", &before, &mut trace);
    let replay = f
        .runtime
        .execute(
            &author(),
            Command::action("flow-a", OBSERVE, "confirmed_permanent".into())
                .at_revision(1)
                .idempotency("confirmed-rejection"),
        )
        .await
        .unwrap();
    record("duplicate_original_outcome_identity", &replay, &mut trace);
    f.drain(&mut trace).await;
    let after = f.read::<Inventory>("sku").await;
    assert_eq!(before.revision, after.revision);
    let duplicate = f
        .act(
            "sku",
            RELEASE,
            "flow-a".into(),
            "new-delivery-same-business-token",
            &mut trace,
        )
        .await;
    assert_eq!(duplicate.revision, before.revision);
    let result = evidence(
        backend,
        "duplicate_outcome_and_clean_reopen",
        "durable_internal_identity_and_semantic_idempotence",
        trace,
        json!({"restart_kind":"clean_close_and_reopen","release_entries":1,"extra_target_revisions_after_replay":0}),
    );
    finish_result(f, result).await
}
async fn external_reopen(backend: &str) -> Value {
    let dir = Scratch::new();
    let path = dir.0.join("db");
    let options = Options::default();
    options
        .provider
        .lock()
        .unwrap()
        .script
        .extend([DeliveryOutcome::Unknown, DeliveryOutcome::Accepted]);
    let mut trace = vec![];
    let f = Fixture::open(backend, &path, options.clone());
    stock_setup(&f, &mut trace, true).await;
    f.act(
        "flow-a",
        DISPATCH,
        "flow-a".into(),
        "provider-request",
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    assert_eq!(options.provider.lock().unwrap().effects.len(), 1);
    f.finish().await;
    options.clock.advance();
    let f = Fixture::open(backend, &path, options.clone());
    f.drain(&mut trace).await;
    {
        let state = options.provider.lock().unwrap();
        assert_eq!(state.deliveries.len(), 2);
        assert_eq!(state.deliveries[0]["id"], state.deliveries[1]["id"]);
        assert_eq!(state.effects.len(), 1);
        trace.push(json!({"step":"receiver_survives_rom_clean_restart_in_test_fixture","deliveries":state.deliveries}));
    }
    let result = evidence(
        backend,
        "external_ack_loss_and_clean_reopen",
        "simulated_receiver_deduplicates_stable_delivery_id",
        trace,
        json!({"external_delivery_attempts":2,"simulated_external_effects":1,"exactly_once_external_claim":false,"provider_persistence":"in_memory_remote_fixture_outlives_ROM_reopen"}),
    );
    finish_result(f, result).await
}
async fn cycle_budget(backend: &str) -> Value {
    let dir = Scratch::new();
    let f = Fixture::open(backend, &dir.0.join("db"), Options::default());
    let mut trace = vec![];
    f.create(
        "loop",
        RecoveryLoop {
            armed: false,
            phase: false,
            count: 0,
        },
        &mut trace,
    )
    .await;
    f.create("flow-a", Workflow::new("cycle", "loop", true), &mut trace)
        .await;
    f.drain(&mut trace).await;
    fail(&f, &mut trace).await;
    f.drain(&mut trace).await;
    let state = f.read::<RecoveryLoop>("loop").await;
    record("bounded_bad_compensation_cycle", &state, &mut trace);
    assert!(f.stopped(StopReason::Depth));
    assert!(state.value.as_ref().unwrap().count > 0 && state.value.as_ref().unwrap().count <= 4);
    assert_eq!(f.runtime.process_work(32).await.unwrap(), 0);
    let result = evidence(
        backend,
        "compensation_cycle_budget",
        "causal_depth_bound_stops_bad_author_declaration",
        trace,
        json!({"stopped":"Depth","successful_flips":state.value.unwrap().count,"initial_failure_fact_preserved":true}),
    );
    finish_result(f, result).await
}
async fn no_terminal_hook(backend: &str) -> Value {
    let dir = Scratch::new();
    let options = Options::default();
    options
        .provider
        .lock()
        .unwrap()
        .script
        .push_back(DeliveryOutcome::Permanent);
    let f = Fixture::open(backend, &dir.0.join("db"), options);
    let mut trace = vec![];
    stock_setup(&f, &mut trace, true).await;
    f.act(
        "flow-a",
        DISPATCH,
        "flow-a".into(),
        "terminal-delivery",
        &mut trace,
    )
    .await;
    f.drain(&mut trace).await;
    assert!(f.stopped(StopReason::DeliveryPermanent));
    let flow = f.read::<Workflow>("flow-a").await;
    let stock = f.read::<Inventory>("sku").await;
    record(
        "worker_failure_is_not_a_domain_failure_fact",
        &flow,
        &mut trace,
    );
    record(
        "reservation_remains_without_failure_classification",
        &stock,
        &mut trace,
    );
    assert_eq!(flow.value.unwrap().outcome, "pending");
    assert!(stock.value.unwrap().reservations.contains_key("flow-a"));
    let result = evidence(
        backend,
        "terminal_work_has_no_automatic_compensation_hook",
        "observed_core_architecture_gap",
        trace,
        json!({"notification_stopped":"DeliveryPermanent","workflow_outcome":"pending","automatic_compensations":0,"required_application_work":"classify_business_outcome_then_record_explicit_failure"}),
    );
    finish_result(f, result).await
}
pub async fn run_backend(backend: &str) -> Vec<Value> {
    let mut cases = vec![];
    for strategy in ["none", "stale_snapshot", "targeted"] {
        cases.push(compare_stock(backend, strategy).await);
        cases.push(compare_seat(backend, strategy).await);
        cases.push(compare_account(backend, strategy).await);
    }
    cases.push(unresolved(backend, false).await);
    cases.push(unresolved(backend, true).await);
    cases.push(compensation_retry(backend, false).await);
    cases.push(compensation_retry(backend, true).await);
    cases.push(revoked(backend).await);
    cases.push(conflict(backend).await);
    cases.push(reopen(backend).await);
    cases.push(external_reopen(backend).await);
    cases.push(cycle_budget(backend).await);
    cases.push(no_terminal_hook(backend).await);
    cases
}
