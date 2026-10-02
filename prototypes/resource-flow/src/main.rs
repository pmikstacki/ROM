//! THROWAWAY loopback host. All routes are kind-independent.
mod declarations;
mod runtime;
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event as SseEvent, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use runtime::{Action, Error, Outcome, Resource, Result, Runtime};
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};

fn principal(headers: &HeaderMap) -> Result<String> {
    // A claimed local principal tests scoping; this is deliberately NOT authentication.
    headers
        .get("x-principal")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && v.len() <= 64)
        .map(str::to_owned)
        .ok_or_else(|| {
            Error(
                StatusCode::UNAUTHORIZED,
                "x-principal is required (prototype identity stub)".into(),
            )
        })
}
async fn schema(State(app): State<Runtime>, headers: HeaderMap) -> Result<Json<serde_json::Value>> {
    principal(&headers)?;
    Ok(Json(serde_json::to_value(&*app.descriptors)?))
}
async fn list(
    State(app): State<Runtime>,
    Path(kind): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<Resource>>> {
    Ok(Json(app.list(principal(&headers)?, kind).await?))
}
async fn read(
    State(app): State<Runtime>,
    Path((kind, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Resource>> {
    Ok(Json(app.get(principal(&headers)?, kind, id).await?))
}
async fn mutation(
    app: Runtime,
    kind: String,
    id: String,
    verb: String,
    headers: HeaderMap,
    request: Action,
) -> Result<Json<Outcome>> {
    let fail = app.faults
        && headers
            .get("x-prototype-fail")
            .and_then(|v| v.to_str().ok())
            == Some("after-event");
    Ok(Json(
        app.execute(principal(&headers)?, kind, id, verb, request, fail)
            .await?,
    ))
}
async fn create(
    State(app): State<Runtime>,
    Path((kind, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(request): Json<Action>,
) -> Result<Json<Outcome>> {
    mutation(app, kind, id, "create".into(), headers, request).await
}
async fn update(
    State(app): State<Runtime>,
    Path((kind, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(request): Json<Action>,
) -> Result<Json<Outcome>> {
    mutation(app, kind, id, "set".into(), headers, request).await
}
async fn delete(
    State(app): State<Runtime>,
    Path((kind, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(request): Json<Action>,
) -> Result<Json<Outcome>> {
    mutation(app, kind, id, "delete".into(), headers, request).await
}
async fn named_action(
    State(app): State<Runtime>,
    Path((kind, id, action)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(request): Json<Action>,
) -> Result<Json<Outcome>> {
    // Built-in verbs are reserved for the generic routes; custom name must be declared.
    let descriptor = app.descriptor(&kind)?;
    if !descriptor.actions.iter().any(|(name, _)| *name == action) {
        return Err(Error(StatusCode::NOT_FOUND, "action not declared".into()));
    }
    mutation(app, kind, id, action, headers, request).await
}
#[derive(Deserialize)]
struct Cursor {
    #[serde(default)]
    after: i64,
}
fn cursor(query: i64, headers: &HeaderMap) -> Result<i64> {
    let after = if let Some(header) = headers.get("last-event-id") {
        header
            .to_str()
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| Error(StatusCode::BAD_REQUEST, "invalid Last-Event-ID".into()))?
    } else {
        query
    };
    if after < 0 {
        Err(Error(StatusCode::BAD_REQUEST, "negative cursor".into()))
    } else {
        Ok(after)
    }
}
async fn journal(
    State(app): State<Runtime>,
    Query(query): Query<Cursor>,
    headers: HeaderMap,
) -> Result<Json<Vec<runtime::Event>>> {
    Ok(Json(
        app.events(Some(principal(&headers)?), cursor(query.after, &headers)?)
            .await?,
    ))
}
async fn events(
    State(app): State<Runtime>,
    Query(query): Query<Cursor>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_core::Stream<Item = std::result::Result<SseEvent, Infallible>>>> {
    let owner = principal(&headers)?;
    let mut after = cursor(query.after, &headers)?;
    let mut hints = app.hints.subscribe(); // Subscribe before reading: no query/subscription gap.
    let stream = async_stream::stream! {
        loop {
            match app.events(Some(owner.clone()),after).await {
                Ok(batch) if !batch.is_empty()=>{
                    for event in batch {
                        after=event.seq;
                        yield Ok(SseEvent::default().id(after.to_string()).event("committed").data(serde_json::to_string(&event).expect("event serialization")));
                    }
                    continue;
                }
                Ok(_)=>{},
                Err(error)=>{
                    yield Ok(SseEvent::default().event("diagnostic").data(serde_json::json!({"error":error.1}).to_string()));
                    break;
                }
            }
            // Lost/lagged hints and process restart are repaired from SQLite; hints are not events.
            tokio::select!{_ = hints.recv()=>{}, _ = tokio::time::sleep(Duration::from_millis(100))=>{}}
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(1))))
}
#[tokio::main]
async fn main() {
    let db = std::env::var("PROTOTYPE_DB").unwrap_or_else(|_| "PROTOTYPE-wipe-me.sqlite".into());
    let app = Runtime::open(&db).expect("open scratch database");
    if std::env::var("PROTOTYPE_REACTIONS").as_deref() != Ok("0") {
        let worker = app.clone();
        tokio::spawn(async move {
            loop {
                match worker.react_once().await {
                    Ok(n) if n > 0 => continue,
                    Ok(_) => {}
                    Err(e) => eprintln!("REACTION diagnostic: {}", e.1),
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }
    let router = Router::new()
        .route("/schema", get(schema))
        .route("/resources/:kind", get(list))
        .route(
            "/resources/:kind/:id",
            get(read).post(create).patch(update).delete(delete),
        )
        .route("/resources/:kind/:id/actions/:action", post(named_action))
        .route("/events", get(events))
        .route("/journal", get(journal))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(app);
    let port = std::env::var("PORT").unwrap_or_else(|_| "7310".into());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .expect("bind loopback");
    println!("LISTENING http://{}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.expect("serve");
}
