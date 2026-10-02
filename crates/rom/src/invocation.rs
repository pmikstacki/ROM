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
    Delete,
    Action { name: String, input: Value },
}
pub(crate) struct ErasedCommand {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) expected: Option<u64>,
    pub(crate) identity: String,
    pub(crate) mutation: Mutation,
}
impl Invocation {
    pub(crate) fn durable_identity(&self, actor: &Actor) -> String {
        let operation = match &self.operation {
            Operation::Create(_) => json!(["standard", "create"]),
            Operation::Replace(_) => json!(["standard", "replace"]),
            Operation::Delete => json!(["standard", "delete"]),
            Operation::Action { name, .. } => json!(["custom", name]),
        };
        json!([
            actor.authority,
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
            identity: self.idempotency,
            mutation: match self.operation {
                Operation::Create(v) => Mutation::Create(v),
                Operation::Replace(v) => Mutation::Replace(v),
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
                Mutation::Delete => Operation::Delete,
                Mutation::Action(name, input) => Operation::Action { name, input },
            },
        }
    }
}
