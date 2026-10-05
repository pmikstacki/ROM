use super::presentation::*;
use crate::{CodecIdentity, Descriptor, FieldCodec, FieldDescriptor, Shape};
use std::collections::BTreeMap;
fn descriptor(shape: Shape) -> Descriptor {
    Descriptor {
        kind: "human".into(),
        version: 1,
        fields: vec![FieldDescriptor {
            name: "name".into(),
            shape,
        }],
    }
}
#[test]
fn title_requires_known_text_without_custom_semantics() {
    let p = ResourcePresentation {
        title_field: Some("name".into()),
        ..Default::default()
    };
    assert!(
        p.validate(&descriptor(Shape::Bool), &BTreeMap::new())
            .is_err()
    );
    assert!(
        p.validate(
            &descriptor(Shape::String),
            &[(
                "name".into(),
                FieldCodec {
                    name: "name".into(),
                    codec: CodecIdentity {
                        name: "opaque".into(),
                        version: 1
                    },
                    codec_wrappers: vec![]
                }
            )]
            .into()
        )
        .is_err()
    );
    assert!(
        p.validate(
            &descriptor(Shape::Optional(Box::new(Shape::Nullable(Box::new(
                Shape::String
            ))))),
            &BTreeMap::new()
        )
        .is_ok()
    );
}
#[test]
fn validates_field_group_and_text_bounds() {
    let descriptor = descriptor(Shape::String);
    for p in [
        ResourcePresentation {
            label: Some(String::new()),
            ..Default::default()
        },
        ResourcePresentation {
            fields: [("secret".into(), FieldPresentation::default())].into(),
            ..Default::default()
        },
        ResourcePresentation {
            fields: [(
                "name".into(),
                FieldPresentation {
                    group: Some("unknown".into()),
                    ..Default::default()
                },
            )]
            .into(),
            ..Default::default()
        },
        ResourcePresentation {
            fields: [(
                "name".into(),
                FieldPresentation {
                    help: Some("é".repeat(1025)),
                    ..Default::default()
                },
            )]
            .into(),
            ..Default::default()
        },
        ResourcePresentation {
            groups: vec![
                PresentationGroup {
                    name: "a".into(),
                    label: "A".into(),
                },
                PresentationGroup {
                    name: "a".into(),
                    label: "B".into(),
                },
            ],
            ..Default::default()
        },
        ResourcePresentation {
            settings: Some(SettingsPresentation {
                group: String::new(),
                label: "Settings".into(),
            }),
            ..Default::default()
        },
    ] {
        assert!(p.validate(&descriptor, &BTreeMap::new()).is_err());
    }
    let p = ResourcePresentation {
        label: Some("é".repeat(128)),
        fields: [(
            "name".into(),
            FieldPresentation {
                help: Some("é".repeat(1024)),
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    };
    assert!(p.validate(&descriptor, &BTreeMap::new()).is_ok());
}
#[test]
fn authoring_json_rejects_unknown_presentation_fields() {
    assert!(
        serde_json::from_str::<ResourcePresentation>(
            r#"{"render":"https://untrusted.example/code.js"}"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<FieldPresentation>(r#"{"format":"email"}"#).is_err());
}
