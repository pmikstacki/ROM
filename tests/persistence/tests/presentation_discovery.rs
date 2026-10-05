//! Presentation is bounded, authorized metadata and never persisted schema identity.
use rom::*;
use std::sync::Arc;

#[derive(Clone, Resource)]
#[resource(name = "presentation-human")]
struct Human {
    name: String,
    secret: String,
}
fn presentation() -> ResourcePresentation {
    ResourcePresentation {
        label: Some("Human".into()),
        title_field: Some("name".into()),
        fields: [
            (
                "name".into(),
                FieldPresentation {
                    label: Some("Full name".into()),
                    group: Some("public".into()),
                    ..Default::default()
                },
            ),
            (
                "secret".into(),
                FieldPresentation {
                    label: Some("Secret label".into()),
                    group: Some("private".into()),
                    ..Default::default()
                },
            ),
        ]
        .into(),
        groups: vec![
            PresentationGroup {
                name: "public".into(),
                label: "Identity".into(),
            },
            PresentationGroup {
                name: "private".into(),
                label: "Secret group".into(),
            },
        ],
        settings: Some(SettingsPresentation {
            group: "people".into(),
            label: "People".into(),
        }),
    }
}
fn build(presentation: ResourcePresentation, title_visible: bool, bytes: usize) -> Result<Runtime> {
    Runtime::builder()
        .limits(Limits {
            snapshot_bytes: bytes,
            ..Limits::default()
        })
        .resource(
            Human::definition()
                .presentation(presentation)
                .discovery_policy(move |_, target| match target {
                    DiscoveryTarget::Field("secret") => false,
                    DiscoveryTarget::Field("name") => title_visible,
                    _ => true,
                }),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:")?),
            Runtime::shared_cpu_pool(1)?,
        )
}
#[tokio::test]
async fn discovers_only_authorized_presentation_and_uses_exact_budget() {
    let actor = Actor::trusted("test", "reader");
    let result = build(presentation(), true, 100_000)
        .unwrap()
        .discover(&actor)
        .await
        .unwrap();
    let encoded = serde_json::to_string(&result).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../studio/tests/fixtures/presentation-discovery.json"
    ))
    .unwrap();
    assert_eq!(serde_json::to_value(&result).unwrap(), fixture);
    assert!(!encoded.contains("Secret"));
    assert!(!encoded.contains("private"));
    let p = result.resources[0].presentation.as_ref().unwrap();
    assert_eq!(p.title_field.as_deref(), Some("name"));
    assert_eq!(p.settings.as_ref().unwrap().group, "people");
    assert_eq!(p.groups.len(), 1);
    assert_eq!(
        build(presentation(), true, encoded.len())
            .unwrap()
            .discover(&actor)
            .await
            .unwrap(),
        result
    );
    assert_eq!(
        build(presentation(), true, encoded.len() - 1)
            .unwrap()
            .discover(&actor)
            .await,
        Err(Error::TooLarge)
    );
    let hidden = build(presentation(), false, 100_000)
        .unwrap()
        .discover(&actor)
        .await
        .unwrap();
    let p = hidden.resources[0].presentation.as_ref().unwrap();
    assert_eq!(p.title_field, None);
    assert!(p.fields.is_empty());
    assert!(p.groups.is_empty());
}
#[test]
fn invalid_presentation_rejects_registration_without_changing_schema() {
    let descriptor = Human::descriptor();
    let mut p = presentation();
    p.title_field = Some("missing".into());
    assert!(matches!(
        build(p, true, 100_000),
        Err(Error::Invalid { .. })
    ));
    let mut p = presentation();
    p.label = Some("é".repeat(129));
    assert!(matches!(
        build(p, true, 100_000),
        Err(Error::Invalid { .. })
    ));
    assert_eq!(Human::descriptor(), descriptor);
}
