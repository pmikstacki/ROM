use super::{
    enum_labels::{field_bindings, validate_labels},
    *,
};
use std::collections::BTreeMap;

#[test]
fn shared_invalid_vectors_fail_label_validation_or_wire_decoding() {
    let cases: crate::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/enum-labels-invalid-v1.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let shape: Shape = serde_json::from_value(case["shape"].clone()).unwrap();
        if let Ok(labels) =
            serde_json::from_value::<BTreeMap<String, String>>(case["enum_labels"].clone())
        {
            assert!(validate_labels(&shape, &labels).is_err());
        }
    }
    let shape = Shape::Enum(vec!["queued".into(), "running".into()]);
    for labels in [
        BTreeMap::from([("queued".into(), "x".repeat(257))]),
        BTreeMap::from([("queued".into(), "é".repeat(129))]),
    ] {
        assert!(validate_labels(&shape, &labels).is_err());
    }
    let labels: BTreeMap<_, _> = (0..1025).map(|n| (n.to_string(), "label".into())).collect();
    assert!(validate_labels(&Shape::Enum(labels.keys().cloned().collect()), &labels).is_err());
    assert!(
        validate_labels(
            &shape,
            &[
                ("queued".into(), "Same".into()),
                ("running".into(), "Same".into())
            ]
            .into()
        )
        .is_ok()
    );
}
#[test]
fn bindings_reject_duplicate_unknown_and_invalid_leaf_metadata() {
    let descriptor = Descriptor {
        kind: "labels".into(),
        version: 1,
        fields: vec![FieldDescriptor {
            name: "phase".into(),
            shape: Shape::Enum(vec!["queued".into()]),
        }],
    };
    let binding = FieldEnumLabels {
        name: "phase".into(),
        labels: [("queued".into(), "Waiting".into())].into(),
    };
    assert!(field_bindings(&descriptor, vec![binding.clone(), binding]).is_err());
    assert!(
        field_bindings(
            &descriptor,
            vec![FieldEnumLabels {
                name: "absent".into(),
                labels: BTreeMap::new()
            }]
        )
        .is_err()
    );
    let wrapped = Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::List(Box::new(
        Shape::Map(Box::new(descriptor.fields[0].shape.clone())),
    ))))));
    assert!(validate_labels(&wrapped, &[("queued".into(), "Waiting".into())].into()).is_ok());
}
