use crate::{Descriptor, Error, Receipt, Result};

impl Receipt {
    /// Check the request codec version for a new commit against its registered schema.
    /// Call only after retained receipt lookup: historical replay may use an older codec.
    /// An absent marker retains the legacy meaning of the current catalog version.
    pub fn validate_new_version(&self, descriptor: &Descriptor) -> Result<()> {
        if self.row.key.kind != descriptor.kind
            || self
                .replay_version
                .is_some_and(|version| version != descriptor.version)
        {
            return Err(Error::invalid(&self.row.key.kind, "replay version"));
        }
        Ok(())
    }
}
