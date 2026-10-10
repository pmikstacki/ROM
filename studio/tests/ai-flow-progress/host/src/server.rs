//! Disposable loopback transport uses host-held authority, never browser-selected principals.
use crate::{authority::owner, state::State as Fixture};
use axum::{Json, Router, extract::{DefaultBodyLimit, State}, http::{HeaderMap, StatusCode}, routing::post};
use rom_ai::{AiError, flow::RunHandle};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::atomic::Ordering, time::Duration};
type Reply = Result<Json<Value>, (StatusCode, Json<Value>)>;
fn fail(error: AiError) -> (StatusCode, Json<Value>) {
    let status = match error { AiError::Denied => StatusCode::FORBIDDEN, AiError::Conflict => StatusCode::CONFLICT, AiError::UnknownOutcome => StatusCode::SERVICE_UNAVAILABLE, _ => StatusCode::BAD_REQUEST };
    (status, Json(json!({"error": error.to_string()})))
}
fn authorize(headers: &HeaderMap, state: &Fixture) -> Result<(), (StatusCode, Json<Value>)> {
    if headers.get("x-fixture-key").and_then(|value| value.to_str().ok()) != Some(state.key.as_str()) { return Err(fail(AiError::Denied)); }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewRequest { #[serde(default)] durable_only: bool }
async fn view(State(state): State<Fixture>, headers: HeaderMap, Json(request): Json<ViewRequest>) -> Reply {
    authorize(&headers, &state)?;
    Ok(Json(serde_json::to_value(state.view(request.durable_only).await.map_err(fail)?).map_err(|_| fail(AiError::Storage))?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation { run_id: String, expected_revision: String, idempotency: String }
async fn operation(state: Fixture, headers: HeaderMap, request: Operation, cancel: bool) -> Reply {
    authorize(&headers, &state)?;
    if request.run_id != "browser-run" { return Err(fail(AiError::Denied)); }
    let revision: u64 = request.expected_revision.parse().map_err(|_| fail(AiError::InvalidRequest))?;
    let handle = RunHandle::new(request.run_id).map_err(fail)?;
    let result = if cancel { state.client.cancel(&owner(), &handle, revision, &request.idempotency).await } else { state.client.resume(&owner(), &handle, revision, &request.idempotency).await };
    Ok(Json(serde_json::to_value(result.map_err(fail)?).map_err(|_| fail(AiError::Storage))?))
}
async fn resume(State(state): State<Fixture>, headers: HeaderMap, Json(request): Json<Operation>) -> Reply { operation(state, headers, request, false).await }
async fn cancel(State(state): State<Fixture>, headers: HeaderMap, Json(request): Json<Operation>) -> Reply { operation(state, headers, request, true).await }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    owner: Option<bool>, tool: Option<bool>, row: Option<bool>,
    #[serde(default)] release: bool,
    #[serde(default)] steps: usize,
    #[serde(default)] until_queued: bool,
}
async fn control(State(state): State<Fixture>, headers: HeaderMap, Json(request): Json<Control>) -> Reply {
    authorize(&headers, &state)?;
    if request.steps > 32 { return Err(fail(AiError::InvalidRequest)); }
    for (value, grant) in [(request.owner, &state.grants.owner), (request.tool, &state.grants.tool), (request.row, &state.grants.row)] {
        if let Some(value) = value { grant.store(value, Ordering::SeqCst); }
    }
    if request.release { state.gate.release().map_err(|_| fail(AiError::Storage))?; }
    let count = if request.until_queued { 24 } else { request.steps };
    for _ in 0..count {
        if request.until_queued && state.view(false).await.map_err(fail)?.read_progress().is_some() { break; }
        state.runtime.process_work(1).await.map_err(|_| fail(AiError::Storage))?;
    }
    Ok(Json(json!({"controlled":true})))
}
async fn inspect(State(state): State<Fixture>, headers: HeaderMap) -> Reply { authorize(&headers, &state)?; Ok(Json(state.inspect().await.map_err(fail)?)) }
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6 { return Err("usage: rom-ai-progress-host ADAPTER DATABASE publication|triage normal|budget|unknown PORT BUDGET_MS".into()); }
    if !matches!(args[2].as_str(), "publication" | "triage") || !matches!(args[3].as_str(), "normal" | "budget" | "unknown") { return Err("invalid domain/mode".into()); }
    let database = PathBuf::from(&args[1]); if !database.is_absolute() { return Err("absolute database required".into()); }
    let budget: u64 = args[5].parse()?; if !(1000..=600000).contains(&budget) { return Err("finite host budget required".into()); }
    let key = std::env::var("ROM_AI_FIXTURE_KEY")?; if key.len() < 32 { return Err("host capability required".into()); }
    let state = Fixture::build(&args[0], &database, args[2] == "publication", &args[3], key).await?;
    let router = Router::new().route("/view", post(view)).route("/resume", post(resume)).route("/cancel", post(cancel)).route("/control", post(control)).route("/inspect", post(inspect))
        .layer(DefaultBodyLimit::max(16384)).with_state(state.clone());
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, args[4].parse::<u16>()?)).await?;
    println!("{}", json!({"address":listener.local_addr()?.to_string(),"adapter":args[0],"domain":args[2]}));
    axum::serve(listener, router).with_graceful_shutdown(async move { tokio::time::sleep(Duration::from_millis(budget)).await; }).await?;
    state.gate.release()?; state.runtime.shutdown().await?;
    Ok(())
}
