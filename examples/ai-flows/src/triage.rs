//! An unrelated application classifies current authorized tickets through one typed action.
use crate::owned::{self, Owned};
use rom::{Action, Field, Resource, Shape, Value};
use rom_ai::{AiError, AiResult, ToolRegistry};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Classification(String);
impl Classification {
    pub fn new(value: &str) -> rom::Result<Self> {
        if matches!(value, "unclassified" | "routine" | "urgent") {
            Ok(Self(value.into()))
        } else {
            Err(rom::Error::invalid("classification", "value"))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Field for Classification {
    fn shape() -> Shape {
        Shape::Enum(vec![
            "unclassified".into(),
            "routine".into(),
            "urgent".into(),
        ])
    }
    fn encode(&self) -> Value {
        Value::String(self.0.clone())
    }
    fn decode(value: Value) -> rom::Result<Self> {
        Self::new(
            value
                .as_str()
                .ok_or_else(|| rom::Error::invalid("classification", "value"))?,
        )
    }
}
#[derive(Clone, Resource)]
#[resource(name = "consumer_triage_tickets", version = 1)]
pub struct Ticket {
    pub owner: String,
    pub body: String,
    pub classification: Classification,
}
#[derive(Clone, rom::Input)]
pub struct Classify {
    pub classification: String,
}
#[derive(Clone, rom::Input)]
struct TicketSource {
    revision: u64,
    body: String,
    classification: Classification,
}
pub const CLASSIFY: Action<Ticket, Classify> = Action::new("classify", |ticket, input| {
    let classification = Classification::new(&input.classification)?;
    if classification.as_str() == "unclassified" {
        return Err(rom::Error::Denied);
    }
    ticket.classification = classification;
    Ok(vec![])
});
impl Owned for Ticket {
    fn owner(&self) -> &str {
        &self.owner
    }
    fn validate(&self) -> rom::Result<()> {
        if self.body.is_empty() || self.body.len() > 4096 {
            Err(rom::Error::TooLarge)
        } else {
            Ok(())
        }
    }
    fn transition(before: Option<&Self>, after: Option<&Self>) -> rom::Result<()> {
        if let (Some(before), Some(after)) = (before, after)
            && before.body != after.body
        {
            Err(rom::Error::Denied)
        } else {
            Ok(())
        }
    }
}
pub fn register(builder: rom::Builder) -> rom::Builder {
    builder.resource(owned::definition::<Ticket>().action(CLASSIFY))
}
pub fn tools(version: u32) -> AiResult<ToolRegistry> {
    let mut registry = ToolRegistry::new(version)?;
    registry.read::<String, TicketSource, _>(
        "read_ticket",
        "Read the current authorized ticket",
        |context, id| {
            Box::pin(async move {
                let snapshot = context
                    .read::<Ticket>(&id)
                    .await
                    .map_err(|_| AiError::Denied)?;
                let ticket = snapshot.value.ok_or(AiError::Denied)?;
                Ok(TicketSource {
                    revision: snapshot.revision,
                    body: ticket.body,
                    classification: ticket.classification,
                })
            })
        },
    )?;
    registry.action(
        "classify_ticket",
        "Apply a validated urgent or routine classification",
        CLASSIFY,
    )?;
    Ok(registry)
}
