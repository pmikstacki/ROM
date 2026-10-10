//! Application declarations shared by the accepted writer and current reader.
use rom::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TicketCode(pub String);

impl Field for TicketCode {
    fn shape() -> Shape {
        Shape::String
    }
    fn codec_identity() -> Option<CodecIdentity> {
        Some(CodecIdentity {
            name: "upgrade-ticket-code".into(),
            version: 1,
        })
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        let text = value
            .as_str()
            .ok_or_else(|| Error::invalid("upgrade-ticket-code", "string"))?;
        let canonical = text.trim().to_ascii_uppercase();
        if canonical.is_empty() || canonical.len() > 32 {
            return Err(Error::invalid("upgrade-ticket-code", "length"));
        }
        Ok(Self(canonical))
    }
}

#[derive(Clone, Debug, Resource)]
#[resource(name = "runtime-upgrade-tickets")]
pub struct Ticket {
    pub code: TicketCode,
    pub quantity: u64,
}

pub const INCREMENT: Action<Ticket, u64> = Action::new("increment", |ticket, amount| {
    ticket.quantity += amount;
    Ok(vec![Intent::new(
        "upgrade-audit",
        json!({"code": ticket.code.0, "quantity": ticket.quantity}),
    )])
});

pub fn definition(allowed: bool) -> Definition<Ticket> {
    let policy: fn(&Actor, Access, &Ticket) -> bool = if allowed {
        |actor, _, _| actor.subject == "owner"
    } else {
        |_, _, _| false
    };
    Ticket::definition()
        .policy(policy)
        .allow_all_fields()
        .action(INCREMENT)
}

pub fn actor() -> Actor {
    Actor::trusted("runtime-upgrade", "owner")
}

pub fn commands() -> [Command<Ticket>; 3] {
    [
        Command::create(
            "one",
            Ticket {
                code: TicketCode(" first ".into()),
                quantity: 1,
            },
        )
        .idempotency("runtime-create"),
        Command::patch(
            "one",
            Patch::new().set(Ticket::code_field(), TicketCode(" second ".into())),
        )
        .at_revision(1)
        .idempotency("runtime-patch"),
        Command::action("one", INCREMENT, 4)
            .at_revision(2)
            .idempotency("runtime-action"),
    ]
}
