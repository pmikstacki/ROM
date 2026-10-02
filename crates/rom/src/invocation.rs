use super::*;

/// Descriptor-routed semantic mutation. Trusted identity is supplied separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub kind: String,
    pub id: String,
    pub expected: Option<u64>,
    pub idempotency: String,
    pub operation: Operation,
}
/// Explicit operation tags keep built-ins distinct from equally named custom actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "input",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Operation {
    Create(Value),
    Replace(Value),
    Patch(BTreeMap<String, FieldUpdate>),
    Delete,
    Action { name: String, input: Value },
}
pub(crate) struct ErasedCommand {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) expected: Option<u64>,
    pub(crate) mutation: Mutation,
}
impl Invocation {
    pub(crate) fn check_size(&self, actor: &Actor, limit: usize) -> Result<()> {
        struct Budget(usize);
        impl std::io::Write for Budget {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 = self
                    .0
                    .checked_sub(bytes.len())
                    .ok_or_else(|| std::io::Error::other("size limit"))?;
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        // Count escaping and routing strings without allocating a serialized request.
        serde_json::to_writer(
            Budget(limit),
            &(
                self,
                &actor.authority,
                actor.principal_kind(),
                &actor.subject,
                actor.host_stamp(),
                &actor.source,
            ),
        )
        .map_err(|_| Error::TooLarge)
    }

    pub(crate) fn durable_identity(&self, actor: &Actor) -> String {
        let operation = match &self.operation {
            Operation::Create(_) => json!(["standard", "create"]),
            Operation::Replace(_) => json!(["standard", "replace"]),
            Operation::Patch(_) => json!(["standard", "patch"]),
            Operation::Delete => json!(["standard", "delete"]),
            Operation::Action { name, .. } => json!(["custom", name]),
        };
        json!([
            actor.authority,
            actor.principal_kind(),
            actor.subject,
            self.kind,
            self.id,
            operation,
            self.idempotency
        ])
        .to_string()
    }

    pub(crate) fn into_command(self) -> ErasedCommand {
        ErasedCommand {
            kind: self.kind,
            id: self.id,
            expected: self.expected,
            mutation: match self.operation {
                Operation::Create(v) => Mutation::Create(v),
                Operation::Replace(v) => Mutation::Replace(v),
                Operation::Patch(v) => Mutation::Patch(v),
                Operation::Delete => Mutation::Delete,
                Operation::Action { name, input } => Mutation::Action(name, input),
            },
        }
    }
}
impl<R: Resource> From<Command<R>> for Invocation {
    fn from(command: Command<R>) -> Self {
        Self {
            kind: R::KIND.into(),
            id: command.id,
            expected: command.expected,
            idempotency: command.identity,
            operation: match command.mutation {
                Mutation::Create(v) => Operation::Create(v),
                Mutation::Replace(v) => Operation::Replace(v),
                Mutation::Patch(v) => Operation::Patch(v),
                Mutation::Delete => Operation::Delete,
                Mutation::Action(name, input) => Operation::Action { name, input },
            },
        }
    }
}
