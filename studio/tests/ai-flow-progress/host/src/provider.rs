//! Deterministic provider drives public tools; no external provider is contacted.
use rom_ai::{AiError, AiFuture, AttemptEvidence, CatalogModel, CatalogSnapshot, Completion, Deadline, MessageRole, ModelPrice, OutputValidator, PreparedAttempt, Provider, ToolCall, Usage, UsdNanos};
use std::sync::atomic::{AtomicU64, Ordering};
pub struct Adapter { pub publication: bool, pub unknown: bool, pub calls: AtomicU64 }
impl Provider for Adapter {
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async { CatalogSnapshot::new("browser-static", vec![CatalogModel::text("fixture/free", 262144, true, true, ModelPrice::free())]) })
    }
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.unknown { return Err(AiError::UnknownOutcome); }
            let evidence = AttemptEvidence::new(attempt.identity(), None, Some(attempt.identity().into()))?;
            let usage = Usage { cost: Some(UsdNanos(0)), ..Default::default() };
            if attempt.request().messages().iter().any(|message| message.role == MessageRole::Tool) {
                return Completion::output(serde_json::json!({"complete":true}), usage, evidence);
            }
            let mut calls = vec![ToolCall::new("probe-read", "probe_read", serde_json::json!({"input": if self.publication { "draft" } else { "ticket" }}))?];
            if self.publication {
                calls.extend([
                    ToolCall::new("read-draft", "read_draft", serde_json::json!({"input":"draft"}))?,
                    ToolCall::new("prepare-edition", "prepare_edition", serde_json::json!({"target":"edition","expected_revision":1,"input":{"draft_revision":1,"text":"First historical article"}}))?,
                    ToolCall::new("publish-head", "publish_head", serde_json::json!({"target":"head","expected_revision":1,"input":"edition"}))?,
                ]);
            } else {
                calls.extend([
                    ToolCall::new("read-ticket", "read_ticket", serde_json::json!({"input":"ticket"}))?,
                    ToolCall::new("classify-ticket", "classify_ticket", serde_json::json!({"target":"ticket","expected_revision":1,"input":{"classification":"urgent"}}))?,
                ]);
            }
            Completion::tool_calls(calls, usage, evidence)
        })
    }
    fn reconcile<'a>(&'a self, _: &'a AttemptEvidence) -> AiFuture<'a, rom_ai::Reconciliation> {
        Box::pin(async { Ok(rom_ai::Reconciliation::Unresolved) })
    }
}
pub struct Validator;
impl OutputValidator for Validator {
    fn validate(&self, value: &serde_json::Value) -> rom_ai::AiResult<serde_json::Value> {
        if value == &serde_json::json!({"complete":true}) { Ok(value.clone()) } else { Err(AiError::InvalidOutput) }
    }
}
