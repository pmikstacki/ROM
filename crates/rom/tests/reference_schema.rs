use rom::{Descriptor, Error, FieldDescriptor, Key, Shape, json};

fn descriptor(shape: Shape) -> Descriptor {
    Descriptor {
        kind: "links".into(),
        version: 1,
        fields: vec![FieldDescriptor {
            name: "targets".into(),
            shape,
        }],
    }
}
fn reference() -> Shape {
    Shape::Reference {
        kind: "items".into(),
    }
}

#[test]
fn nested_references_are_unique_and_preserve_full_identity() {
    let schema = descriptor(Shape::Map(Box::new(Shape::List(Box::new(
        Shape::Nullable(Box::new(reference())),
    )))));
    let targets = schema
        .reference_targets(Some(&json!({
            "targets": {"first": ["b", null, "a"], "second": ["a"]}
        })))
        .unwrap();
    assert_eq!(
        targets,
        vec![
            Key {
                kind: "items".into(),
                id: "a".into()
            },
            Key {
                kind: "items".into(),
                id: "b".into()
            },
        ]
    );
}

#[test]
fn absence_null_and_tombstones_have_no_live_edges() {
    let schema = descriptor(Shape::Optional(Box::new(Shape::Nullable(Box::new(
        reference(),
    )))));
    for value in [None, Some(json!({})), Some(json!({"targets":null}))] {
        assert!(schema.reference_targets(value.as_ref()).unwrap().is_empty());
    }
    assert!(
        descriptor(reference())
            .reference_targets(None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn malformed_reference_values_fail_instead_of_omitting_edges() {
    let schema = descriptor(reference());
    for value in [
        json!({}),
        json!({"targets":null}),
        json!({"targets":""}),
        json!({"targets":17}),
        json!({"targets":"a","extra":"b"}),
    ] {
        assert!(schema.reference_targets(Some(&value)).is_err());
    }
    assert_eq!(
        schema.reference_targets(Some(&json!({"targets":17}))),
        Err(Error::invalid("links", "targets"))
    );
}

#[test]
fn descriptors_round_trip_and_reject_invalid_layouts() {
    let schema = descriptor(reference());
    let restored: Descriptor =
        serde_json::from_str(&serde_json::to_string(&schema).unwrap()).unwrap();
    assert_eq!(schema, restored);
    let mut invalid = schema.clone();
    invalid.fields.push(invalid.fields[0].clone());
    assert!(
        invalid
            .reference_targets(Some(&json!({"targets":"a"})))
            .is_err()
    );
    let mut deep = reference();
    for _ in 0..18 {
        deep = Shape::List(Box::new(deep));
    }
    assert!(descriptor(deep).reference_targets(None).is_err());
}

#[test]
fn catalog_validation_checks_targets_duplicates_and_canonical_order() {
    let source = descriptor(reference());
    assert!(rom::validate_descriptors(std::slice::from_ref(&source)).is_err());
    let target = Descriptor {
        kind: "items".into(),
        version: 1,
        fields: vec![],
    };
    assert!(rom::validate_descriptors(&[source.clone(), source.clone(), target.clone()]).is_err());
    let catalog = rom::validate_descriptors(&[source.clone(), target.clone()]).unwrap();
    assert_eq!(catalog, vec![target, source]);
    let mut schema = descriptor(Shape::String);
    schema.fields.push(FieldDescriptor {
        name: "a".into(),
        shape: Shape::Bool,
    });
    assert_eq!(schema.canonical().unwrap().fields[0].name, "a");
    schema.version = 0;
    assert!(schema.canonical().is_err());
}
