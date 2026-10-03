use super::*;

fn row(id: &str) -> Row {
    Row {
        key: Key {
            kind: "notes".into(),
            id: id.into(),
        },
        revision: 7,
        value: Some(json!({"old": id})),
        protected: ProtectedMetadata {
            deletion_authorization: Some(json!({"old": "deleted"})),
            source_provenance: Some(SourceProvenance {
                source: "importer".into(),
                version: "v1".into(),
                generation: 9,
                field_origins: BTreeMap::from([("old".into(), "remote".into())]),
            }),
        },
    }
}
fn pending(id: &str, payload: WorkPayload) -> PendingWork {
    PendingWork {
        id: id.into(),
        cause: Cause {
            retry_epoch: 0,
            root: "root".into(),
            parent: Some("parent".into()),
            depth: 3,
            started_at: 10,
            path: vec!["stage".into()],
        },
        definition: "notify".into(),
        version: 4,
        service_key: "service".into(),
        payload,
    }
}
fn rename(_: &Key, value: &Value) -> Result<Value> {
    Ok(json!({"new": value["old"]}))
}
fn fixture() -> StorageState {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state.head = 1;
    state.receipts = 1;
    state.events.push(JournalEvent {
        position: 1,
        identity: "receipt".into(),
        row: row("journal"),
    });
    state
        .work
        .enqueue(
            &ReactionLimits::default(),
            vec![
                pending(
                    "a-notice",
                    WorkPayload::Notification {
                        source: row("notice"),
                        payload: json!({"old": "wire payload"}),
                    },
                ),
                pending("b-source", WorkPayload::Source(row("source"))),
                pending(
                    "c-action",
                    WorkPayload::Action(json!({"old": "action input"})),
                ),
            ],
        )
        .unwrap();
    let WorkResult::Claimed(claim) = state.work.apply(WorkUpdate::Claim { now: 10 }).unwrap()
    else {
        panic!("expected work")
    };
    state
        .work
        .apply(WorkUpdate::DeliveryStarted {
            claim: claim.key(),
            now: 10,
        })
        .unwrap();
    state
}

#[test]
fn row_mapping_changes_only_resource_values_and_is_atomic_on_error() {
    let original = row("one");
    let mapped = original.map_resource_values(&mut rename).unwrap();
    let mut expected = original.clone();
    expected.value = Some(json!({"new": "one"}));
    expected.protected.deletion_authorization = Some(json!({"new": "deleted"}));
    assert_eq!(mapped, expected);
    assert_eq!(original, row("one"));
    let mut seen = 0;
    assert_eq!(
        original.map_resource_values(&mut |key, _| {
            assert_eq!(key, &original.key);
            seen += 1;
            if seen == 2 {
                Err(Error::Storage)
            } else {
                Ok(json!("changed"))
            }
        }),
        Err(Error::Storage)
    );
    assert_eq!(original, row("one"));
}

#[test]
fn tombstone_mapping_does_not_restore_a_live_value() {
    let mut original = row("deleted");
    original.value = None;
    let mapped = original.map_resource_values(&mut rename).unwrap();
    assert!(mapped.value.is_none());
    assert_eq!(
        mapped.protected.deletion_authorization,
        Some(json!({"new": "deleted"}))
    );
}

#[test]
fn state_mapping_preserves_obligation_identity_lifecycle_and_opaque_payloads() {
    let mut state = fixture();
    let mut expected = serde_json::to_value(&state).unwrap();
    expected["events"][0]["row"]["value"] = json!({"new": "journal"});
    expected["events"][0]["row"]["protected"]["deletion_authorization"] = json!({"new": "deleted"});
    expected["work"]["work"]["a-notice"]["pending"]["payload"]["Notification"]["source"]["value"] =
        json!({"new": "notice"});
    expected["work"]["work"]["a-notice"]["pending"]["payload"]["Notification"]["source"]["protected"]
        ["deletion_authorization"] = json!({"new": "deleted"});
    expected["work"]["work"]["b-source"]["pending"]["payload"]["Source"]["value"] =
        json!({"new": "source"});
    expected["work"]["work"]["b-source"]["pending"]["payload"]["Source"]["protected"]["deletion_authorization"] =
        json!({"new": "deleted"});
    state.map_resource_values(&mut rename).unwrap();
    assert_eq!(serde_json::to_value(&state).unwrap(), expected);
}

#[test]
fn state_mapping_rolls_back_journal_and_work_if_a_later_transform_fails() {
    let mut state = fixture();
    let before = serde_json::to_value(&state).unwrap();
    let error = state.map_resource_values(&mut |key, value| {
        if key.id == "source" {
            Err(Error::Denied)
        } else {
            rename(key, value)
        }
    });
    assert_eq!(error, Err(Error::Denied));
    assert_eq!(serde_json::to_value(&state).unwrap(), before);
}

#[test]
fn journal_expansion_rejects_without_eviction_or_partial_changes() {
    let mut state = fixture();
    state.limits.journal_bytes = serde_json::to_vec(&state.events[0]).unwrap().len() + 16;
    let before = serde_json::to_value(&state).unwrap();
    assert_eq!(
        state.map_resource_values(&mut |_, _| Ok(json!("x".repeat(4096)))),
        Err(Error::TooLarge)
    );
    assert_eq!(serde_json::to_value(&state).unwrap(), before);
}

#[test]
fn work_expansion_preserves_lifecycle_capacity_and_rolls_back() {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    let limits = ReactionLimits {
        max_bytes: 2000,
        ..ReactionLimits::default()
    };
    state
        .work
        .enqueue(
            &limits,
            vec![pending("source", WorkPayload::Source(row("one")))],
        )
        .unwrap();
    let before = serde_json::to_value(&state).unwrap();
    assert_eq!(
        state.map_resource_values(&mut |_, _| Ok(json!("x".repeat(4096)))),
        Err(Error::Overloaded)
    );
    assert_eq!(serde_json::to_value(&state).unwrap(), before);
}

#[test]
fn empty_state_never_calls_transform() {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    let before = serde_json::to_value(&state).unwrap();
    state
        .map_resource_values(&mut |_, _| panic!("no Resource values"))
        .unwrap();
    assert_eq!(serde_json::to_value(&state).unwrap(), before);
}

#[test]
fn mapping_rejects_growth_that_fits_current_bytes_but_consumes_lifecycle_reserve() {
    let mut expanded_row = row("one");
    expanded_row.value = Some(json!("x".repeat(500)));
    expanded_row.protected.deletion_authorization = Some(json!("x".repeat(500)));
    let expanded_work = pending("source", WorkPayload::Source(expanded_row));
    let mut expanded = WorkLedger::default();
    expanded
        .enqueue(&ReactionLimits::default(), vec![expanded_work.clone()])
        .unwrap();
    let current_bytes = serde_json::to_vec(&expanded).unwrap().len();
    let limits = ReactionLimits {
        max_bytes: current_bytes + 16,
        ..ReactionLimits::default()
    };
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state
        .work
        .enqueue(
            &limits,
            vec![pending("source", WorkPayload::Source(row("one")))],
        )
        .unwrap();
    let before = serde_json::to_value(&state).unwrap();
    // Use a serialized witness to prove actual candidate bytes fit this limit.
    let mut candidate = serde_json::to_value(&state.work).unwrap();
    candidate["work"]["source"]["pending"] = serde_json::to_value(expanded_work).unwrap();
    assert!(serde_json::to_vec(&candidate).unwrap().len() < limits.max_bytes);
    assert_eq!(
        state.map_resource_values(&mut |_, _| Ok(json!("x".repeat(500)))),
        Err(Error::Overloaded)
    );
    assert_eq!(serde_json::to_value(&state).unwrap(), before);
}
