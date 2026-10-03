//! Typed offline representation changes. Business changes still use Resource actions.
use rom::{Descriptor, Error, PendingWork, Resource, Result, Value};

/// One exact Resource schema transition with a pure, bounded native converter.
/// Keys, revisions and action inputs are outside this conversion contract.
pub struct ResourceMigration {
    pub(crate) before: Descriptor,
    pub(crate) after: Descriptor,
    pub(crate) convert: Box<dyn Fn(Value) -> Result<Value> + Send + Sync>,
}
impl ResourceMigration {
    /// Define one consecutive version step for the same Resource kind.
    pub fn new<Before: Resource, After: Resource>(
        convert: fn(Before) -> Result<After>,
    ) -> Result<Self> {
        let before = Before::descriptor().canonical()?;
        let after = After::descriptor().canonical()?;
        if before.kind != Before::KIND
            || after.kind != After::KIND
            || before.kind != after.kind
            || before.version.checked_add(1) != Some(after.version)
        {
            return Err(Error::Unsupported(
                "migration must advance the same Resource by one version".into(),
            ));
        }
        Ok(Self {
            before,
            after,
            convert: Box::new(move |value| {
                let source = Before::decode(value.clone())?;
                if source.encode() != value {
                    return Err(Error::invalid(Before::KIND, "migration source codec"));
                }
                let output = convert(source)?.encode();
                if After::decode(output.clone())?.encode() != output {
                    return Err(Error::invalid(After::KIND, "migration target codec"));
                }
                Ok(output)
            }),
        })
    }
}
/// Ordered schema steps. A validator must accept each unfinished durable obligation.
/// Native callbacks must be pure and bounded; they do not run in a sandbox.
pub struct MigrationPlan {
    pub(crate) steps: Vec<ResourceMigration>,
    pub(crate) work_validator: Option<fn(&PendingWork) -> Result<()>>,
}
impl MigrationPlan {
    pub fn new(steps: Vec<ResourceMigration>) -> Result<Self> {
        if steps.is_empty() {
            return Err(Error::Unsupported(
                "migration requires a version step".into(),
            ));
        }
        Ok(Self {
            steps,
            work_validator: None,
        })
    }
    /// Check frozen reaction/action/channel contracts against the destination application.
    /// Returning success attests compatibility; this callback cannot rewrite an obligation.
    pub fn validate_work(mut self, validate: fn(&PendingWork) -> Result<()>) -> Self {
        self.work_validator = Some(validate);
        self
    }
}
