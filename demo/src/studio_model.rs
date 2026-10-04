//! Held-out declaration and codec identities for the explicit Studio demo extension.
use rom::{Action, CodecIdentity, Field, Resource, Result, Shape, Value};

#[derive(Clone)]
pub struct TicketCode(String);
impl Field for TicketCode {
    fn shape() -> Shape {
        Shape::String
    }
    fn codec_identity() -> Option<CodecIdentity> {
        Some(CodecIdentity {
            name: "demo-ticket-code".into(),
            version: 1,
        })
    }
    fn encode(&self) -> Value {
        self.0.clone().into()
    }
    fn decode(value: Value) -> Result<Self> {
        let text = value
            .as_str()
            .ok_or_else(|| rom::Error::invalid("code", "string required"))?
            .trim()
            .to_ascii_uppercase();
        if !text.starts_with("TICKET-")
            || text.len() < 8
            || text.len() > 32
            || !text
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err(rom::Error::invalid("code", "TICKET- code required"));
        }
        Ok(Self(text))
    }
}
#[derive(Clone)]
pub struct OpaqueHandle(String);
impl Field for OpaqueHandle {
    fn shape() -> Shape {
        Shape::String
    }
    fn codec_identity() -> Option<CodecIdentity> {
        Some(CodecIdentity {
            name: "demo-opaque-handle".into(),
            version: 1,
        })
    }
    fn encode(&self) -> Value {
        self.0.clone().into()
    }
    fn decode(value: Value) -> Result<Self> {
        let text = value
            .as_str()
            .ok_or_else(|| rom::Error::invalid("opaque", "string required"))?;
        if text.is_empty() || text.len() > 64 {
            return Err(rom::Error::invalid("opaque", "bounded handle required"));
        }
        Ok(Self(text.into()))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "maintenance-tickets")]
pub struct MaintenanceTicket {
    pub code: TicketCode,
    pub optional_code: Option<TicketCode>,
    pub code_list: Vec<TicketCode>,
    pub summary: String,
    pub open: bool,
    pub attempts: u64,
    pub opaque: Option<OpaqueHandle>,
    pub required_handle: OpaqueHandle,
}
pub const CLOSE: Action<MaintenanceTicket, ()> = Action::new("close", |ticket, ()| {
    ticket.open = false;
    Ok(vec![])
});
