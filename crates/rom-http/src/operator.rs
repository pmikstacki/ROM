//! HTTP binding for the core's authorized work inspection and recovery contract.
use crate::{
    error::Failure,
    request::{decode, decode_empty},
    server::Shared,
};
use axum::{
    Json,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use rom::operator::{WorkControlRequest, WorkHandle, WorkQuery};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    handle: WorkHandle,
}

pub(super) async fn capabilities(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (actor, _permit) = decode_empty(&s, r).await?;
    Ok(Json(s.runtime.operator_capabilities(&actor).await?).into_response())
}

pub(super) async fn list(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (actor, query, _permit) = decode::<WorkQuery>(&s, r).await?;
    Ok(Json(s.runtime.work_list(&actor, query).await?).into_response())
}

pub(super) async fn read(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (actor, request, _permit) = decode::<Read>(&s, r).await?;
    Ok(Json(s.runtime.work_read(&actor, request.handle).await?).into_response())
}

pub(super) async fn control(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (actor, request, _permit) = decode::<WorkControlRequest>(&s, r).await?;
    Ok(Json(s.runtime.work_control(&actor, request).await?).into_response())
}
