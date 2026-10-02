#[derive(Debug, PartialEq, typed_builder::TypedBuilder)]
pub struct ResourceConfig {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default = 3)]
    pub retries: u32,
    // Keep Option explicit: callers can pass values computed conditionally.
    #[builder(default)]
    pub description: Option<String>,
}

#[derive(Debug, PartialEq, typed_builder::TypedBuilder)]
pub struct ActionInput {
    #[builder(setter(into))]
    pub resource_id: String,
    pub expected_revision: u64,
    #[builder(default)]
    pub note: Option<Option<String>>,
}

// Tuple positions track which fields have been supplied.
pub fn retry_policy<N, D>(
    builder: ResourceConfigBuilder<(N, (), D)>,
    production: bool,
) -> ResourceConfigBuilder<(N, (u32,), D)> {
    builder.retries(if production { 5 } else { 3 })
}

pub fn verify() {
    let production = true;
    let config = retry_policy(ResourceConfig::builder().name("sensor"), production)
        .description(production.then(|| "production".to_owned()))
        .build();
    assert_eq!(config.retries, 5);
    assert_eq!(config.description.as_deref(), Some("production"));
    assert_eq!(ResourceConfig::builder().name("default").build().retries, 3);
    let absent = ActionInput::builder()
        .resource_id("s1")
        .expected_revision(0)
        .build();
    let null = ActionInput::builder()
        .resource_id("s1")
        .expected_revision(0)
        .note(Some(None))
        .build();
    let value = ActionInput::builder()
        .resource_id("s1")
        .expected_revision(0)
        .note(Some(Some("ok".to_owned())))
        .build();
    assert_eq!(absent.note, None);
    assert_eq!(null.note, Some(None));
    assert_eq!(value.note, Some(Some("ok".to_owned())));
}

// Convenience setters can retain an explicit Option escape hatch.
#[derive(typed_builder::TypedBuilder)]
struct OptionalInput {
    #[builder(default, setter(strip_option(fallback = note_option)))]
    note: Option<String>,
}

pub fn verify_optional_fallback() {
    assert_eq!(
        OptionalInput::builder().note_option(None).build().note,
        None
    );
    assert_eq!(
        OptionalInput::builder()
            .note("ok".to_owned())
            .build()
            .note
            .as_deref(),
        Some("ok")
    );
}
