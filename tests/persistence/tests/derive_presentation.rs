//! Derived and manual presentation use the same registration and disclosure contract.
use rom::*;
use std::sync::Arc;

#[derive(Clone, Resource)]
#[resource(
    name = "derive-presentation",
    label = "People",
    title_field = "display-name",
    settings(group = "people", label = "People"),
    group(name = "identity", label = "Identity")
)]
struct Human {
    #[resource(
        rename = "display-name",
        label = "Full name",
        help = "Shown in lists",
        group = "identity"
    )]
    name: Option<String>,
    #[resource(label = "Private note")]
    secret: String,
}
fn build<R: Resource>(definition: Definition<R>) -> Result<Runtime> {
    Runtime::builder().resource(definition).build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:")?),
        Runtime::shared_cpu_pool(1)?,
    )
}
#[tokio::test]
async fn renamed_derived_hints_follow_authorized_fields_and_preserve_codec() {
    let human = Human {
        name: Some("Ada".into()),
        secret: "private".into(),
    };
    let value = json!({"display-name":"Ada", "secret":"private"});
    assert_eq!(human.encode(), value);
    assert_eq!(Human::decode(value.clone()).unwrap().encode(), value);
    assert_eq!(Human::descriptor().fields[0].name, "display-name");
    let runtime = build(
        Human::definition()
            .discovery_policy(|_, target| !matches!(target, DiscoveryTarget::Field("secret"))),
    )
    .unwrap();
    let discovery = runtime
        .discover(&Actor::trusted("test", "reader"))
        .await
        .unwrap();
    let metadata = discovery.resources[0].presentation.as_ref().unwrap();
    assert_eq!(metadata.title_field.as_deref(), Some("display-name"));
    assert_eq!(metadata.fields.len(), 1);
    assert_eq!(
        metadata.fields["display-name"].label.as_deref(),
        Some("Full name")
    );
    assert_eq!(
        metadata.fields["display-name"].help.as_deref(),
        Some("Shown in lists")
    );
    assert_eq!(metadata.groups[0].name, "identity");
    assert_eq!(metadata.settings.as_ref().unwrap().group, "people");

    let runtime = build(
        Human::definition()
            .discovery_policy(|_, target| !matches!(target, DiscoveryTarget::Field(_))),
    )
    .unwrap();
    let discovery = runtime
        .discover(&Actor::trusted("test", "reader"))
        .await
        .unwrap();
    let metadata = discovery.resources[0].presentation.as_ref().unwrap();
    assert_eq!(metadata.title_field, None);
    assert!(metadata.fields.is_empty());
    assert!(metadata.groups.is_empty());
}

#[derive(Clone)]
struct CustomText(String);
impl Field for CustomText {
    fn shape() -> Shape {
        Shape::String
    }
    fn codec_identity() -> Option<CodecIdentity> {
        Some(CodecIdentity {
            name: "custom-title".into(),
            version: 1,
        })
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        value
            .as_str()
            .map(|text| Self(text.into()))
            .ok_or_else(|| Error::invalid("custom-title", "$"))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "derived-custom-title", title_field = "title")]
struct DerivedCustom {
    title: CustomText,
}
#[derive(Clone)]
struct ManualCustom {
    title: CustomText,
}
impl Resource for ManualCustom {
    const KIND: &'static str = "manual-custom-title";
    fn descriptor() -> Descriptor {
        Descriptor {
            kind: Self::KIND.into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "title".into(),
                shape: Shape::String,
            }],
        }
    }
    fn field_codecs() -> Vec<FieldCodec> {
        vec![FieldCodec {
            name: "title".into(),
            codec: CustomText::codec_identity().unwrap(),
            codec_wrappers: vec![],
        }]
    }
    fn presentation() -> Option<ResourcePresentation> {
        Some(ResourcePresentation {
            title_field: Some("title".into()),
            ..Default::default()
        })
    }
    fn normalize_field(name: &str, value: Value) -> Result<Value> {
        match name {
            "title" => <CustomText as Field>::decode(value).map(|value| Field::encode(&value)),
            _ => Err(Error::invalid(Self::KIND, name)),
        }
    }
    fn encode(&self) -> Value {
        json!({"title":Field::encode(&self.title)})
    }
    fn decode(value: Value) -> Result<Self> {
        let Value::Object(mut fields) = value else {
            return Err(Error::invalid(Self::KIND, "$"));
        };
        let title = <CustomText as Field>::decode(
            fields
                .remove("title")
                .ok_or_else(|| Error::invalid(Self::KIND, "title"))?,
        )?;
        if !fields.is_empty() {
            return Err(Error::invalid(Self::KIND, "unknown field"));
        }
        Ok(Self { title })
    }
}
#[test]
fn custom_title_semantics_reject_derived_and_manual_registration() {
    for result in [
        build(DerivedCustom::definition()),
        build(ManualCustom::definition()),
    ] {
        assert!(matches!(result, Err(Error::Invalid { field, .. }) if field == "presentation"));
    }
}
// Aliases cannot be classified at expansion time; registration checks the real Field shape.
type HiddenBoolean = bool;
#[derive(Clone, Resource)]
#[resource(name = "aliased-title", title_field = "title")]
struct AliasedTitle {
    title: HiddenBoolean,
}
#[test]
fn aliased_title_shape_remains_a_registration_error() {
    assert!(
        matches!(build(AliasedTitle::definition()), Err(Error::Invalid { field, .. }) if field == "presentation")
    );
}
#[derive(Clone, Resource)]
#[resource(name = "plain-presentation")]
struct Plain {
    name: String,
}
#[test]
fn absent_presentation_preserves_default_and_builder_override_is_validated() {
    assert_eq!(Plain::presentation(), None);
    assert!(build(Plain::definition()).is_ok());
    let metadata = ResourcePresentation {
        label: Some("Override".into()),
        title_field: Some("name".into()),
        ..Default::default()
    };
    assert!(build(Plain::definition().presentation(metadata)).is_ok());
    let invalid = ResourcePresentation {
        title_field: Some("missing".into()),
        ..Default::default()
    };
    assert!(
        matches!(build(Human::definition().presentation(invalid)), Err(Error::Invalid { field, .. }) if field == "presentation")
    );
}
