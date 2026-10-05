use rom::Resource;
#[derive(Clone, Resource)]
#[resource(
    name = "human",
    crate = "::rom",
    label = "Human resources",
    title_field = "display-name",
    settings(group = "studio", label = "Studio"),
    group(name = "details", label = "Details")
)]
struct Human {
    #[resource(
        rename = "display-name",
        label = "Display name",
        help = "Shown in lists",
        group = "details"
    )]
    name: Option<String>,
    enabled: bool,
}
fn main() {
    let metadata = Human::presentation().unwrap();
    assert_eq!(metadata.label.as_deref(), Some("Human resources"));
    assert_eq!(metadata.title_field.as_deref(), Some("display-name"));
    assert!(!metadata.fields.contains_key("name"));
    assert_eq!(
        metadata.fields["display-name"].label.as_deref(),
        Some("Display name")
    );
    assert_eq!(
        metadata.fields["display-name"].help.as_deref(),
        Some("Shown in lists")
    );
    assert_eq!(
        metadata.fields["display-name"].group.as_deref(),
        Some("details")
    );
    assert_eq!(metadata.groups[0].label, "Details");
    assert_eq!(metadata.settings.unwrap().group, "studio");
    let value = Human {
        name: Some("Ada".into()),
        enabled: false,
    };
    assert_eq!(
        value.encode(),
        rom::json!({"display-name":"Ada","enabled":false})
    );
    assert_eq!(
        Human::decode(value.encode()).unwrap().name.as_deref(),
        Some("Ada")
    );
    assert_eq!(Human::descriptor().fields[0].name, "display-name");
}
