use super::input_descriptor::field_bindings;
use super::*;
use crate::{Field, Input, json};

#[derive(Clone, crate::Input)]
#[input(crate = "crate")]
struct Named {
    #[input(rename = "wire-name")]
    value: bool,
    count: Option<u64>,
}

#[test]
fn typed_inputs_describe_their_actual_wire_values() {
    assert_eq!(<() as Input>::descriptor(), Some(InputDescriptor::Unit));
    assert_eq!(
        <u64 as Input>::descriptor(),
        Some(InputDescriptor::Scalar {
            codec_wrappers: vec![],
            shape: Shape::U64,
            codec: None
        })
    );
    let InputDescriptor::Object(fields) = Named::descriptor().unwrap() else {
        panic!("object required")
    };
    assert_eq!(fields[0].name, "wire-name");
    assert_eq!(fields[0].shape, bool::shape());
    assert_eq!(fields[1].shape, Option::<u64>::shape());
    assert_eq!(
        Named {
            value: false,
            count: Some(u64::MAX)
        }
        .encode(),
        json!({"wire-name":false,"count":u64::MAX})
    );
}

#[test]
fn descriptors_reject_ambiguous_or_invalid_metadata() {
    let field = InputFieldDescriptor {
        codec_wrappers: vec![],
        name: "same".into(),
        shape: Shape::Bool,
        codec: None,
    };
    assert!(
        InputDescriptor::Object(vec![field.clone(), field])
            .validate(None)
            .is_err()
    );
    assert!(
        CodecIdentity {
            name: String::new(),
            version: 1
        }
        .validate()
        .is_err()
    );
    assert!(
        CodecIdentity {
            name: "money".into(),
            version: 0
        }
        .validate()
        .is_err()
    );
    assert!(
        InputDescriptor::Scalar {
            codec_wrappers: vec![],
            shape: Shape::Optional(Box::new(Shape::Bool)),
            codec: None
        }
        .validate(None)
        .is_err()
    );
}

#[test]
fn field_bindings_reject_duplicate_unknown_and_invalid_codec_identities() {
    let layout = crate::Descriptor {
        kind: "items".into(),
        version: 1,
        fields: vec![crate::FieldDescriptor {
            name: "amount".into(),
            shape: Shape::String,
        }],
    };
    let binding = FieldCodec {
        codec_wrappers: vec![],
        name: "amount".into(),
        codec: CodecIdentity {
            name: "money".into(),
            version: 1,
        },
    };
    assert_eq!(
        field_bindings(&layout, vec![binding.clone()]).unwrap()["amount"].codec,
        binding.codec
    );
    assert!(field_bindings(&layout, vec![binding.clone(), binding.clone()]).is_err());
    assert!(
        field_bindings(
            &layout,
            vec![FieldCodec {
                codec_wrappers: vec![],
                name: "unknown".into(),
                ..binding.clone()
            }]
        )
        .is_err()
    );
    assert!(
        field_bindings(
            &layout,
            vec![FieldCodec {
                codec_wrappers: vec![],
                codec: CodecIdentity {
                    name: "money".into(),
                    version: 0
                },
                ..binding
            }]
        )
        .is_err()
    );
}

#[test]
fn standalone_presence_keeps_its_existing_tagged_codec_and_is_opaque() {
    assert_eq!(<crate::Presence<bool> as Input>::descriptor(), None);
    assert_eq!(
        <crate::Presence<bool> as Input>::decode(crate::json!({"presence":"missing"})).unwrap(),
        crate::Presence::Missing
    );
}

#[derive(Clone)]
struct ManualNamed(Named);
impl Input for ManualNamed {
    fn descriptor() -> Option<InputDescriptor> {
        Some(InputDescriptor::Object(vec![
            InputFieldDescriptor {
                codec_wrappers: vec![],
                name: "wire-name".into(),
                shape: Shape::Bool,
                codec: None,
            },
            InputFieldDescriptor {
                codec_wrappers: vec![],
                name: "count".into(),
                shape: Shape::Nullable(Box::new(Shape::U64)),
                codec: None,
            },
        ]))
    }
    fn encode(&self) -> crate::Value {
        Input::encode(&self.0)
    }
    fn decode(value: crate::Value) -> crate::Result<Self> {
        Named::decode(value).map(Self)
    }
}
#[test]
fn manual_and_derived_inputs_share_metadata_and_codec_results() {
    assert_eq!(ManualNamed::descriptor(), Named::descriptor());
    let value = Named {
        value: false,
        count: Some(u64::MAX),
    }
    .encode();
    assert_eq!(ManualNamed::decode(value.clone()).unwrap().encode(), value);
    assert!(ManualNamed::decode(crate::json!({"wire-name":"false","count":null})).is_err());
}

#[derive(Clone, crate::Input)]
#[input(crate = "crate")]
struct Flexible {
    flag: crate::Presence<Option<bool>>,
}
#[test]
fn derived_optional_nullable_input_members_preserve_missing_null_and_false() {
    let InputDescriptor::Object(fields) = Flexible::descriptor().unwrap() else {
        panic!("object required")
    };
    assert_eq!(
        fields[0].shape,
        Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::Bool))))
    );
    let missing = Flexible::decode(crate::json!({})).unwrap();
    assert_eq!(missing.encode(), crate::json!({}));
    let null = Flexible::decode(crate::json!({"flag":null})).unwrap();
    assert_eq!(null.encode(), crate::json!({"flag":null}));
    let value = Flexible::decode(crate::json!({"flag":false})).unwrap();
    assert_eq!(value.encode(), crate::json!({"flag":false}));
}

#[derive(Clone)]
struct WrappedLabel(String);
impl Field for WrappedLabel {
    fn shape() -> Shape {
        Shape::String
    }
    fn codec_identity() -> Option<CodecIdentity> {
        Some(CodecIdentity {
            name: "wrapped-label".into(),
            version: 1,
        })
    }
    fn encode(&self) -> crate::Value {
        Field::encode(&self.0)
    }
    fn decode(value: crate::Value) -> crate::Result<Self> {
        <String as Field>::decode(value).map(Self)
    }
}
#[derive(Clone, crate::Input)]
#[input(crate = "crate")]
struct NestedCodecInput {
    labels: Vec<Option<WrappedLabel>>,
    optional_labels: crate::Presence<Vec<Option<WrappedLabel>>>,
}
#[test]
fn builtin_wrappers_preserve_leaf_codec_and_explicit_presentation_path() {
    assert_eq!(
        Option::<WrappedLabel>::codec_identity(),
        WrappedLabel::codec_identity()
    );
    let metadata = serde_json::to_value(NestedCodecInput::descriptor().unwrap()).unwrap();
    assert_eq!(metadata["value"][0]["codec"]["name"], "wrapped-label");
    assert_eq!(
        metadata["value"][0]["codec_wrappers"],
        json!(["list", "nullable"])
    );
    assert_eq!(metadata["value"][1]["codec"]["name"], "wrapped-label");
    assert_eq!(
        metadata["value"][1]["codec_wrappers"],
        json!(["optional", "list", "nullable"])
    );
}

#[test]
fn codec_wrapper_paths_must_match_the_declared_shape() {
    let layout = crate::Descriptor {
        kind: "items".into(),
        version: 1,
        fields: vec![crate::FieldDescriptor {
            name: "label".into(),
            shape: Shape::String,
        }],
    };
    let invalid = FieldCodec {
        name: "label".into(),
        codec: CodecIdentity {
            name: "wrapped-label".into(),
            version: 1,
        },
        codec_wrappers: vec![CodecWrapper::List],
    };
    assert!(field_bindings(&layout, vec![invalid]).is_err());
    let no_codec = InputDescriptor::Scalar {
        shape: Shape::List(Box::new(Shape::String)),
        codec: None,
        codec_wrappers: vec![CodecWrapper::List],
    };
    assert!(no_codec.validate(None).is_err());
    let valid = InputDescriptor::Scalar {
        shape: Shape::Map(Box::new(Shape::List(Box::new(Shape::Nullable(Box::new(
            Shape::String,
        )))))),
        codec: WrappedLabel::codec_identity(),
        codec_wrappers: vec![
            CodecWrapper::Map,
            CodecWrapper::List,
            CodecWrapper::Nullable,
        ],
    };
    assert!(valid.validate(None).is_ok());
}
