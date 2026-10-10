//! Fixture identity and revocable current tool/row authority are selected by the host.
use rom::{Actor, Resource};
use rom_ai::{AiError, AiResult, PreparedAttempt, ToolCall, flow::{FlowAuthority, OwnerIdentity, Submission}};
use rom_ai_flows_consumer::{publication::{Draft, Edition, Head}, triage::Ticket};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

pub fn owner() -> Actor { Actor::trusted("external-ai-consumer", "author") }
pub fn service() -> Actor { Actor::trusted("external-ai-consumer", "ai-service").with_kind(rom::PrincipalKind::Service) }
#[derive(Default)]
pub struct Grants { pub owner: AtomicBool, pub tool: AtomicBool, pub row: AtomicBool }
impl Grants {
    pub fn allowed() -> Self { Self { owner: AtomicBool::new(true), tool: AtomicBool::new(true), row: AtomicBool::new(true) } }
}
pub struct Authority(pub Arc<Grants>);
impl FlowAuthority for Authority {
    fn submit(&self, actor: &Actor, _: &Submission) -> AiResult<OwnerIdentity> {
        let identity = OwnerIdentity::from_actor(actor)?;
        self.inspect(actor, &identity)?;
        Ok(identity)
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if self.0.owner.load(Ordering::SeqCst) && identity.matches(actor) && identity.matches(&owner()) { Ok(()) } else { Err(AiError::Denied) }
    }
    fn cancel(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> { self.inspect(actor, identity) }
    fn resolve(&self, identity: &OwnerIdentity, _: &mut dyn rom::AuthorizationRead) -> rom::Result<Actor> {
        self.inspect(&owner(), identity).map_err(|_| rom::Error::Denied)?;
        Ok(owner())
    }
    fn attempt(&self, actor: &Actor, identity: &OwnerIdentity, prepared: &PreparedAttempt, _: &mut dyn rom::AuthorizationRead) -> rom::Result<()> {
        self.inspect(actor, identity).map_err(|_| rom::Error::Denied)?;
        prepared.validate().map_err(|_| rom::Error::Denied)?;
        if prepared.route().maximum_cost().0 != 0 { return Err(rom::Error::Denied); }
        Ok(())
    }
    fn tool_actor(&self, actor: &Actor, identity: &OwnerIdentity, call: &ToolCall, reads: &mut dyn rom::AuthorizationRead) -> rom::Result<Actor> {
        self.inspect(actor, identity).map_err(|_| rom::Error::Denied)?;
        if !self.0.tool.load(Ordering::SeqCst) || !self.0.row.load(Ordering::SeqCst) { return Err(rom::Error::Denied); }
        let args = call.arguments();
        let (kind, id) = match call.name() {
            "probe_read" => if args["input"] == "draft" { (Draft::KIND, Some("draft")) } else if args["input"] == "ticket" { (Ticket::KIND, Some("ticket")) } else { return Err(rom::Error::Denied); },
            "read_draft" => (Draft::KIND, args["input"].as_str()),
            "prepare_edition" => (Edition::KIND, args["target"].as_str()),
            "publish_head" => (Head::KIND, args["target"].as_str()),
            "read_ticket" => (Ticket::KIND, args["input"].as_str()),
            "classify_ticket" => (Ticket::KIND, args["target"].as_str()),
            _ => return Err(rom::Error::Denied),
        };
        let value = reads.load(&rom::Key { kind: kind.into(), id: id.ok_or(rom::Error::Denied)?.into() })?.and_then(|row| row.value).ok_or(rom::Error::Denied)?;
        if value["owner"].as_str() != Some(actor.subject.as_str()) { return Err(rom::Error::Denied); }
        if call.name() == "prepare_edition" {
            let source = reads.load(&rom::Key { kind: Draft::KIND.into(), id: value["draft"].as_str().ok_or(rom::Error::Denied)?.into() })?.and_then(|row| row.value).ok_or(rom::Error::Denied)?;
            if source["owner"].as_str() != Some(actor.subject.as_str()) { return Err(rom::Error::Denied); }
        }
        if call.name() == "publish_head" {
            let edition = reads.load(&rom::Key { kind: Edition::KIND.into(), id: args["input"].as_str().ok_or(rom::Error::Denied)?.into() })?.and_then(|row| row.value).ok_or(rom::Error::Denied)?;
            if edition["owner"].as_str() != Some(actor.subject.as_str()) || edition["prepared"] != true { return Err(rom::Error::Denied); }
        }
        Ok(actor.clone())
    }
}
