//! Exact-version request interpretation for already committed receipts.
use crate::*;

/// A frozen legacy definition is used only for its codecs, never its policies or actions.
pub(crate) struct Codec {
    definition: Arc<dyn Registered>,
    kind: &'static str,
}
impl Codec {
    pub(crate) fn new<R: Resource>() -> Self {
        Self {
            definition: Arc::new(Definition::<R>::new()),
            kind: R::KIND,
        }
    }
    pub(crate) fn descriptor(&self) -> &Descriptor {
        self.definition.descriptor_ref()
    }
    pub(crate) fn validate_for(&self, current: &Descriptor) -> Result<()> {
        let old = self.descriptor();
        old.canonical()?;
        if old.kind != self.kind || old.kind != current.kind || old.version >= current.version {
            return Err(Error::invalid(&current.kind, "replay version"));
        }
        Ok(())
    }
}

pub(crate) fn input(definition: &dyn Registered, mutation: &Mutation) -> Result<Value> {
    match mutation {
        Mutation::Create(value) | Mutation::Replace(value) => definition.normalize(value.clone()),
        Mutation::Patch(fields) => {
            serde_json::to_value(normalize_patch(definition, fields)?).map_err(|_| Error::Storage)
        }
        Mutation::Delete => Ok(Value::Null),
        Mutation::Action(_, value) => Ok(value.clone()),
    }
}

pub(crate) fn fingerprint(expected: Option<u64>, input: Value, actor: &Actor) -> String {
    match &actor.source {
        Some(permit) => json!([expected, input, permit.provenance]).to_string(),
        None => json!([expected, input]).to_string(),
    }
}

impl Runtime {
    /// Call under the runtime gate. Current disclosure precedes request codecs.
    pub(crate) fn replay_outcome(
        &self,
        actor: &Actor,
        definition: &dyn Registered,
        current: Option<&Row>,
        receipt: Receipt,
        command: &invocation::ErasedCommand,
    ) -> Result<Row> {
        self.disclose(actor, &(), definition, current, &receipt.row)?;
        let current_version = definition.descriptor_ref().version;
        let version = receipt.replay_version.unwrap_or(current_version);
        let codec = if version == current_version {
            definition
        } else {
            definition
                .replay_codec(version)
                .ok_or_else(|| Error::Unsupported("receipt replay codec unavailable".into()))?
                .definition
                .as_ref()
        };
        // An application codec must not unwind through the held authorization gate.
        let input = catch_unwind(AssertUnwindSafe(|| input(codec, &command.mutation)))
            .unwrap_or(Err(Error::Panicked))?;
        if receipt.fingerprint != fingerprint(command.expected, input, actor) {
            return Err(Error::IdentityMismatch);
        }
        Ok(receipt.row)
    }
}
