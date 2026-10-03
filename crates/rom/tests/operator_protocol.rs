use rom::operator::*;
use rom::{Error, json};

fn request() -> rom::Value {
    json!({"handle": WorkHandle::from_work_id("private work identity"), "expected": {"generation":"storage-1", "revision":4}, "key":"request-1", "retry_epoch":0, "operation":{"Reconcile":{"evidence_ref":"provider-reference"}}})
}
#[test]
fn requests_reject_unknown_fields_at_each_boundary() {
    let valid = request();
    assert!(serde_json::from_value::<WorkControlRequest>(valid.clone()).is_ok());
    for field in ["request", "version", "operation"] {
        let mut invalid = valid.clone();
        match field {
            "request" => invalid["payload"] = json!("secret"),
            "version" => invalid["expected"]["claim_generation"] = json!(1),
            _ => invalid["operation"]["Reconcile"]["accepted"] = json!(true),
        }
        assert!(serde_json::from_value::<WorkControlRequest>(invalid).is_err());
    }
}
#[test]
fn control_identifiers_and_evidence_are_bounded_before_submission() {
    for (field, oversized) in [
        ("key", "a".repeat(1025)),
        ("evidence", "b".repeat(1025)),
        ("generation", "c".repeat(129)),
    ] {
        let mut invalid = request();
        match field {
            "key" => invalid["key"] = json!(oversized),
            "evidence" => invalid["operation"]["Reconcile"]["evidence_ref"] = json!(oversized),
            _ => invalid["expected"]["generation"] = json!(oversized),
        }
        assert!(serde_json::from_value::<WorkControlRequest>(invalid).is_err());
    }
    let mut invalid = request();
    invalid["key"] = json!("");
    assert!(serde_json::from_value::<WorkControlRequest>(invalid).is_err());
}
#[test]
fn query_and_response_bounds_reject_zero_oversized_and_overflow() {
    let limits = WorkResponseLimits::default();
    let query: WorkQuery = serde_json::from_value(json!({"limit":64})).unwrap();
    query.validate(&limits).unwrap();
    assert!(serde_json::from_value::<WorkQuery>(json!({"limit":0})).is_err());
    assert!(serde_json::from_value::<WorkQuery>(json!({"limit":1,"payload":true})).is_err());
    let large: WorkQuery = serde_json::from_value(json!({"limit":65})).unwrap();
    assert!(matches!(large.validate(&limits), Err(Error::TooLarge)));
    assert!(
        WorkResponseLimits {
            max_records: 0,
            max_bytes: 1
        }
        .validate()
        .is_err()
    );
    assert!(
        WorkResponseLimits {
            max_records: usize::MAX,
            max_bytes: usize::MAX
        }
        .validate()
        .is_err()
    );
    let page = WorkPage {
        protocol_version: OPERATOR_PROTOCOL_VERSION,
        records: vec![],
        cursor: None,
    };
    assert!(matches!(
        page.validate(&WorkResponseLimits {
            max_records: 1,
            max_bytes: 1
        }),
        Err(Error::TooLarge)
    ));
}
#[test]
fn handles_are_stable_opaque_and_canonical() {
    let handle = WorkHandle::from_work_id("secret identity");
    let wire = serde_json::to_string(&handle).unwrap();
    assert!(!wire.contains("secret"));
    assert_eq!(handle, WorkHandle::from_work_id("secret identity"));
    assert_ne!(handle, WorkHandle::from_work_id("other identity"));
    assert!(serde_json::from_value::<WorkHandle>(json!("secret identity")).is_err());
    assert!(serde_json::from_value::<WorkHandle>(json!("A".repeat(64))).is_err());
    assert!(serde_json::from_value::<WorkCursor>(json!("x".repeat(2049))).is_err());
}

fn view() -> WorkView {
    WorkView {
        protocol_version: OPERATOR_PROTOCOL_VERSION,
        handle: WorkHandle::from_work_id("work"),
        version: WorkVersion {
            generation: "storage-1".into(),
            revision: 4,
        },
        category: WorkCategory::Notification,
        definition: WorkDefinition {
            name: "notice".into(),
            version: 1,
        },
        state: WorkStatus::Pending,
        attempts: 0,
        due: 10,
        delivery: None,
        source: None,
        target: None,
    }
}
#[test]
fn page_rejects_nested_fields_its_json_decoder_cannot_accept() {
    for field in ["generation", "definition", "empty_definition"] {
        let mut invalid = view();
        match field {
            "generation" => invalid.version.generation = "g".repeat(129),
            "definition" => invalid.definition.name = "d".repeat(1025),
            _ => invalid.definition.name.clear(),
        }
        let page = WorkPage {
            protocol_version: OPERATOR_PROTOCOL_VERSION,
            records: vec![invalid],
            cursor: None,
        };
        assert!(serde_json::from_value::<WorkPage>(serde_json::to_value(&page).unwrap()).is_err());
        assert!(matches!(
            page.validate(&WorkResponseLimits::default()),
            Err(Error::TooLarge)
        ));
    }
}
#[test]
fn page_rejects_wrong_protocol_on_envelope_and_nested_view() {
    for nested in [false, true] {
        let mut page = WorkPage {
            protocol_version: OPERATOR_PROTOCOL_VERSION,
            records: vec![view()],
            cursor: None,
        };
        if nested {
            page.records[0].protocol_version += 1;
        } else {
            page.protocol_version += 1;
        }
        assert!(page.validate(&WorkResponseLimits::default()).is_err());
        assert!(serde_json::from_value::<WorkPage>(serde_json::to_value(page).unwrap()).is_err());
    }
}
#[test]
fn persisted_reconciliation_hold_has_a_public_safe_status() {
    assert!(serde_json::from_value::<WorkStatus>(json!("AwaitingReconciliation")).is_ok());
}

#[test]
fn individual_outputs_validate_nested_fields_and_complete_envelope_bytes() {
    let valid = view();
    valid.validate(&WorkResponseLimits::default()).unwrap();
    let bytes = serde_json::to_vec(&valid).unwrap().len();
    valid
        .validate(&WorkResponseLimits {
            max_records: 1,
            max_bytes: bytes,
        })
        .unwrap();
    assert!(matches!(
        valid.validate(&WorkResponseLimits {
            max_records: 1,
            max_bytes: bytes - 1
        }),
        Err(Error::TooLarge)
    ));
    let mut result = WorkControlResult {
        protocol_version: OPERATOR_PROTOCOL_VERSION,
        handle: valid.handle,
        version: valid.version,
        key: "request-1".into(),
        operation: WorkControlOperation::Retry,
        outcome: WorkControlOutcome::Scheduled,
        replayed: false,
    };
    result.validate(&WorkResponseLimits::default()).unwrap();
    let bytes = serde_json::to_vec(&result).unwrap().len();
    assert!(matches!(
        result.validate(&WorkResponseLimits {
            max_records: 1,
            max_bytes: bytes - 1
        }),
        Err(Error::TooLarge)
    ));
    for field in ["generation", "key", "evidence", "protocol"] {
        let mut invalid = result.clone();
        match field {
            "generation" => invalid.version.generation = "g".repeat(129),
            "key" => invalid.key = "k".repeat(1025),
            "evidence" => {
                invalid.operation = WorkControlOperation::Reconcile {
                    evidence_ref: Some("e".repeat(1025)),
                }
            }
            _ => invalid.protocol_version += 1,
        }
        assert!(invalid.validate(&WorkResponseLimits::default()).is_err());
        assert!(
            serde_json::from_value::<WorkControlResult>(serde_json::to_value(&invalid).unwrap())
                .is_err()
        );
    }
    result.protocol_version += 1;
    assert!(result.validate(&WorkResponseLimits::default()).is_err());
    let mut capabilities = OperatorCapabilities::default();
    capabilities
        .validate(&WorkResponseLimits::default())
        .unwrap();
    capabilities.protocol_version += 1;
    assert!(
        capabilities
            .validate(&WorkResponseLimits::default())
            .is_err()
    );
    assert!(
        serde_json::from_value::<OperatorCapabilities>(serde_json::to_value(capabilities).unwrap())
            .is_err()
    );
}
#[test]
fn page_charges_full_envelope_and_record_count_after_nested_validation() {
    let page = WorkPage {
        protocol_version: OPERATOR_PROTOCOL_VERSION,
        records: vec![view(), view()],
        cursor: Some(WorkCursor::try_from("cursor".to_owned()).unwrap()),
    };
    let bytes = serde_json::to_vec(&page).unwrap().len();
    page.validate(&WorkResponseLimits {
        max_records: 2,
        max_bytes: bytes,
    })
    .unwrap();
    assert!(matches!(
        page.validate(&WorkResponseLimits {
            max_records: 1,
            max_bytes: bytes
        }),
        Err(Error::TooLarge)
    ));
    assert!(matches!(
        page.validate(&WorkResponseLimits {
            max_records: 2,
            max_bytes: bytes - 1
        }),
        Err(Error::TooLarge)
    ));
}
