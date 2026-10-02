#[derive(Debug, PartialEq)]
pub struct ResourceConfig {
    pub name: String,
    pub retries: u32,
    pub description: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct ActionInput {
    pub resource_id: String,
    pub expected_revision: u64,
    pub note: Option<Option<String>>,
}

#[derive(Debug, PartialEq)]
pub struct MissingField(pub &'static str);

pub struct ResourceConfigBuilder {
    name: Option<String>,
    retries: u32,
    description: Option<String>,
}

impl ResourceConfig {
    pub fn builder() -> ResourceConfigBuilder {
        ResourceConfigBuilder {
            name: None,
            retries: 3,
            description: None,
        }
    }
}

impl ResourceConfigBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }
    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }
    pub fn build(self) -> Result<ResourceConfig, MissingField> {
        Ok(ResourceConfig {
            name: self.name.ok_or(MissingField("name"))?,
            retries: self.retries,
            description: self.description,
        })
    }
}

#[derive(Default)]
pub struct ActionInputBuilder {
    resource_id: Option<String>,
    expected_revision: Option<u64>,
    note: Option<Option<String>>,
}

impl ActionInput {
    pub fn builder() -> ActionInputBuilder {
        ActionInputBuilder::default()
    }
}

impl ActionInputBuilder {
    pub fn resource_id(mut self, id: impl Into<String>) -> Self {
        self.resource_id = Some(id.into());
        self
    }
    pub fn expected_revision(mut self, revision: u64) -> Self {
        self.expected_revision = Some(revision);
        self
    }
    pub fn note(mut self, note: Option<Option<String>>) -> Self {
        self.note = note;
        self
    }
    pub fn build(self) -> Result<ActionInput, MissingField> {
        Ok(ActionInput {
            resource_id: self.resource_id.ok_or(MissingField("resource_id"))?,
            expected_revision: self
                .expected_revision
                .ok_or(MissingField("expected_revision"))?,
            note: self.note,
        })
    }
}

pub fn retry_policy(builder: ResourceConfigBuilder) -> ResourceConfigBuilder {
    builder.retries(5)
}

pub fn verify() {
    let production = true;
    let mut builder = retry_policy(ResourceConfig::builder().name("sensor"));
    if production {
        builder = builder.description(Some("production".to_owned()));
    }
    let config = builder.build().unwrap();
    assert_eq!(config.retries, 5);
    assert_eq!(config.description.as_deref(), Some("production"));
    assert_eq!(
        ResourceConfig::builder()
            .name("default")
            .build()
            .unwrap()
            .retries,
        3
    );
    let absent = ActionInput::builder()
        .resource_id("s1")
        .expected_revision(0)
        .build()
        .unwrap();
    let null = ActionInput::builder()
        .resource_id("s1")
        .expected_revision(0)
        .note(Some(None))
        .build()
        .unwrap();
    let value = ActionInput::builder()
        .resource_id("s1")
        .expected_revision(0)
        .note(Some(Some("ok".to_owned())))
        .build()
        .unwrap();
    assert_eq!(absent.note, None);
    assert_eq!(null.note, Some(None));
    assert_eq!(value.note, Some(Some("ok".to_owned())));
}
