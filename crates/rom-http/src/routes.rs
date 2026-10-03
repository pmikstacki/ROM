use crate::{
    error::Failure,
    observation::observe,
    request::{decode, decode_empty},
    server::Shared,
};
use axum::{
    Json,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use rom::{Invocation, JournalCursor, Value};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    kind: String,
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EqualityQuery {
    kind: String,
    field: String,
    value: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructuredQuery {
    kind: String,
    query: rom::QuerySpec,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Query {
    Structured(Box<StructuredQuery>),
    Equality(EqualityQuery),
}
impl Query {
    fn into_spec(self) -> (String, rom::QuerySpec) {
        match self {
            Self::Structured(q) => (q.kind, q.query),
            Self::Equality(q) => (q.kind, rom::QuerySpec::equal(q.field, q.value)),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    kind: String,
    after: Option<JournalCursor>,
}
pub(super) async fn invoke(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Invocation>(&s, r).await?;
    Ok(Json(s.runtime.invoke_projected(&a, c).await?).into_response())
}
pub(super) async fn discover(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (actor, _permit) = decode_empty(&s, r).await?;
    Ok(Json(s.runtime.discover(&actor).await?).into_response())
}
pub(super) async fn read(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Read>(&s, r).await?;
    Ok(Json(s.runtime.read_projected(&a, &c.kind, &c.id).await?).into_response())
}
pub(super) async fn query(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Query>(&s, r).await?;
    let (kind, spec) = c.into_spec();
    Ok(Json(s.runtime.query_spec_projected(&a, &kind, spec).await?).into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalHead {
    kind: String,
}
pub(super) async fn journal_head(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<JournalHead>(&s, r).await?;
    Ok(Json(s.runtime.journal_head(&a, &c.kind).await?).into_response())
}
pub(super) async fn journal(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Journal>(&s, r).await?;
    Ok(Json(s.runtime.journal(&a, &c.kind, c.after.as_ref()).await?).into_response())
}
pub(super) async fn live(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Query>(&s, r).await?;
    let (kind, spec) = c.into_spec();
    let handle = s.runtime.live_spec_projected(&a, &kind, spec).await?;
    Ok(observe(s, a, handle, |handle| {
        Box::pin(async move { handle.changed().await })
    }))
}
pub(super) async fn subscribe(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Journal>(&s, r).await?;
    let handle = s.runtime.subscribe(&a, &c.kind, c.after).await?;
    Ok(observe(s, a, handle, |handle| {
        Box::pin(async move { handle.next().await })
    }))
}
