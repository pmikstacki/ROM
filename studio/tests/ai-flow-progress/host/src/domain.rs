//! Reuse maintained public domain registration, typed tools and actions.
use crate::{authority::owner, gate::ReadGate};
use rom::{Command, Runtime};
use rom_ai::{AiError, AiResult, ToolRegistry};
use rom_ai_flows_consumer::{publication::{self, Draft, Edition, Head}, triage::{self, Classification, Ticket}};
use std::{sync::Arc, time::Duration};

pub fn builder(publication: bool) -> rom::Builder {
    if publication { publication::register(Runtime::builder()) } else { triage::register(Runtime::builder()) }
}
pub fn tools(publication: bool, gate: Arc<ReadGate>) -> AiResult<ToolRegistry> {
    let mut registry = if publication { publication::tools(1)? } else { triage::tools(1)? };
    registry.read::<String, bool, _>("probe_read", "Bounded fixture-only pure read ownership probe", move |context, _| {
        let gate = gate.clone();
        Box::pin(async move {
            tokio::time::timeout(Duration::from_millis(150), context.calculate(move || gate.wait())).await
                .map_err(|_| AiError::UnknownOutcome)?.map_err(|_| AiError::UnknownOutcome)
        })
    })?;
    Ok(registry)
}
pub async fn seed(runtime: &Runtime, publication: bool) -> rom::Result<()> {
    if publication {
        runtime.execute(&owner(), Command::create("draft", Draft { owner: "author".into(), text: "First historical article".into() }).idempotency("browser-create-draft")).await?;
        runtime.execute(&owner(), Command::create("edition", Edition { owner: "author".into(), draft: rom::ResourceRef::new("draft")?, draft_revision: 1, text: "First historical article".into(), prepared: false }).idempotency("browser-capture-edition")).await?;
        runtime.execute(&owner(), Command::create("head", Head { owner: "author".into(), edition: None }).idempotency("browser-create-head")).await?;
    } else {
        runtime.execute(&owner(), Command::create("ticket", Ticket { owner: "author".into(), body: "Pump requires inspection".into(), classification: Classification::new("unclassified")? }).idempotency("browser-create-ticket")).await?;
    }
    Ok(())
}
pub async fn inspect(runtime: &Runtime, publication: bool) -> rom::Result<serde_json::Value> {
    if publication {
        let edition = runtime.read::<Edition>(&owner(), "edition").await?;
        let head = runtime.read::<Head>(&owner(), "head").await?;
        Ok(serde_json::json!({ "edition_revision": edition.revision, "prepared": edition.value.map(|value| value.prepared), "head_revision": head.revision, "published": head.value.is_some_and(|value| value.edition.is_some()) }))
    } else {
        let ticket = runtime.read::<Ticket>(&owner(), "ticket").await?;
        Ok(serde_json::json!({ "ticket_revision": ticket.revision, "classification": ticket.value.map(|value| value.classification.as_str().to_owned()) }))
    }
}
