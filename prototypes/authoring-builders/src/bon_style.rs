#[derive(Debug, PartialEq, bon::Builder)]
pub struct ResourceConfig {
    #[builder(into)]
    pub name: String,
    #[builder(default = 3)]
    pub retries: u32,
    pub description: Option<String>,
}

#[derive(Debug, PartialEq, bon::Builder)]
pub struct ActionInput {
    #[builder(into)]
    pub resource_id: String,
    pub expected_revision: u64,
    /// None = absent, Some(None) = explicit null, Some(Some(value)) = value.
    pub note: Option<Option<String>>,
}

// A helper can accept any state provided retries has not been set yet.
pub fn retry_policy<S>(
    builder: ResourceConfigBuilder<S>,
    production: bool,
) -> ResourceConfigBuilder<resource_config_builder::SetRetries<S>>
where
    S: resource_config_builder::State,
    S::Retries: resource_config_builder::IsUnset,
{
    builder.retries(if production { 5 } else { 3 })
}

pub fn verify() {
    let production = true;
    let description = production.then(|| "production".to_owned());
    let config = retry_policy(ResourceConfig::builder().name("sensor"), production)
        .maybe_description(description)
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
        .note(None)
        .build();
    let value = ActionInput::builder()
        .resource_id("s1")
        .expected_revision(0)
        .note(Some("ok".to_owned()))
        .build();
    assert_eq!(absent.note, None);
    assert_eq!(null.note, Some(None));
    assert_eq!(value.note, Some(Some("ok".to_owned())));
}
