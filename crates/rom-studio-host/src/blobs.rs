//! Authenticated binary transport. BlobService owns accepted work and Resource mutations.
use crate::{
    authentication, csrf,
    router::{Shared, no_store},
};
use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use rom_blob::{BlobService, Digest, Error, Upload, UploadOutcome};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reserve {
    id: String,
    store: String,
    digest: String,
    bytes: u64,
    idempotency: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Target {
    id: String,
}

pub(crate) async fn authorize(
    shared: &Arc<Shared>,
    headers: &axum::http::HeaderMap,
    mutation: bool,
) -> Result<rom::Actor, Box<Response>> {
    let cookie = csrf::session_cookie(headers, "rom_session")
        .map_err(|_| Box::new(authentication::denied(StatusCode::UNAUTHORIZED)))?;
    let session = shared
        .sessions
        .lookup(&cookie, shared.config.clock.now())
        .ok_or_else(|| Box::new(authentication::denied(StatusCode::UNAUTHORIZED)))?;
    if mutation {
        csrf::check_origin(headers, &shared.config.public_origin)
            .map_err(|_| Box::new(authentication::denied(StatusCode::UNAUTHORIZED)))?;
        csrf::check_token(headers, &session)
            .map_err(|_| Box::new(authentication::denied(StatusCode::FORBIDDEN)))?;
    }
    authentication::resolve(shared, &cookie)
        .await
        .map_err(|error| Box::new(authentication::failure(error)))
}
fn service(shared: &Shared) -> Result<BlobService, Box<Response>> {
    shared
        .config
        .blobs
        .clone()
        .ok_or_else(|| Box::new(failure(Error::Unsupported)))
}
async fn input<T: serde::de::DeserializeOwned>(
    shared: &Shared,
    request: Request,
) -> Result<T, Box<Response>> {
    let _permit = shared
        .blob_bodies
        .clone()
        .try_acquire_owned()
        .map_err(|_| Box::new(failure(Error::Overloaded)))?;
    let body = tokio::time::timeout(
        shared.config.http_limits.body_timeout,
        axum::body::to_bytes(request.into_body(), shared.config.http_limits.body_bytes),
    )
    .await
    .map_err(|_| Box::new(failure(Error::Timeout)))?
    .map_err(|_| Box::new(failure(Error::TooLarge)))?;
    serde_json::from_slice(&body).map_err(|_| Box::new(failure(Error::Invalid)))
}
pub(crate) async fn reserve(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let actor = match authorize(&shared, request.headers(), true).await {
        Ok(a) => a,
        Err(e) => return *e,
    };
    let service = match service(&shared) {
        Ok(s) => s,
        Err(e) => return *e,
    };
    let input: Reserve = match input(&shared, request).await {
        Ok(i) => i,
        Err(e) => return *e,
    };
    let digest = match Digest::parse(&input.digest) {
        Ok(d) => d,
        Err(e) => return failure(e),
    };
    match service
        .reserve(
            &actor,
            &input.id,
            &input.store,
            digest,
            input.bytes,
            &input.idempotency,
        )
        .await
    {
        Ok(_) => projected(&shared, &actor, &input.id, false).await,
        Err(error) => failure(error),
    }
}
pub(crate) async fn upload(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let actor = match authorize(&shared, request.headers(), true).await {
        Ok(a) => a,
        Err(e) => return *e,
    };
    let service = match service(&shared) {
        Ok(s) => s,
        Err(e) => return *e,
    };
    let target = match target(&request) {
        Ok(t) => t,
        Err(e) => return *e,
    };
    let upload = chunks(request.into_body(), service.limits());
    match service.upload(&actor, &target.id, upload).await {
        Ok(UploadOutcome::Attached(_)) => projected(&shared, &actor, &target.id, true).await,
        // Provider publication can exist without a confirmed Resource attachment.
        // A receipt is not field-projection authority. Keep it out of browser responses.
        Ok(UploadOutcome::Unattached { .. }) => failure(Error::Unknown),
        Err(error) => failure(error),
    }
}
async fn projected(shared: &Shared, actor: &rom::Actor, id: &str, attached: bool) -> Response {
    match shared.runtime.read_projected(actor, "blobs", id).await {
        Ok(view) => no_store(axum::Json(serde_json::json!({"status": if attached {"attached"} else {"reserved"}, "resource":view})).into_response()),
        // The mutation may already be committed. Never report this as a pre-invoke rejection.
        Err(_) => failure(Error::Unknown),
    }
}
pub(crate) async fn download(
    State(shared): State<Arc<Shared>>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    let actor = match authorize(&shared, request.headers(), false).await {
        Ok(a) => a,
        Err(e) => return *e,
    };
    attachment(&shared, &actor, &id).await
}
pub(crate) async fn download_query(
    State(shared): State<Arc<Shared>>,
    request: Request,
) -> Response {
    let actor = match authorize(&shared, request.headers(), false).await {
        Ok(a) => a,
        Err(e) => return *e,
    };
    let target = match target(&request) {
        Ok(t) => t,
        Err(e) => return *e,
    };
    attachment(&shared, &actor, &target.id).await
}
fn target(request: &Request) -> Result<Target, Box<Response>> {
    Query::<Target>::try_from_uri(request.uri())
        .map(|Query(target)| target)
        .map_err(|_| Box::new(failure(Error::Invalid)))
}
async fn attachment(shared: &Shared, actor: &rom::Actor, id: &str) -> Response {
    let service = match service(shared) {
        Ok(s) => s,
        Err(e) => return *e,
    };
    match service.read(actor, id).await {
        Ok(bytes) => no_store(
            (
                [
                    (header::CONTENT_TYPE, "application/octet-stream"),
                    (header::CONTENT_DISPOSITION, "attachment"),
                    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                ],
                bytes,
            )
                .into_response(),
        ),
        Err(error) => failure(error),
    }
}
pub(crate) async fn detach(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let actor = match authorize(&shared, request.headers(), true).await {
        Ok(a) => a,
        Err(e) => return *e,
    };
    let service = match service(&shared) {
        Ok(s) => s,
        Err(e) => return *e,
    };
    let input: Target = match input(&shared, request).await {
        Ok(i) => i,
        Err(e) => return *e,
    };
    match service.detach(&actor, &input.id).await {
        Ok(_) => {
            match shared
                .runtime
                .read_projected(&actor, "blobs", &input.id)
                .await
            {
                Ok(view) => no_store(
                    axum::Json(serde_json::json!({"status":"detached","resource":view}))
                        .into_response(),
                ),
                Err(_) => failure(Error::Unknown),
            }
        }
        Err(error) => failure(error),
    }
}
fn chunks(body: Body, limits: rom_blob::Limits) -> Upload {
    Box::pin(futures_util::stream::try_unfold(
        (body.into_data_stream(), Bytes::new()),
        move |(mut input, mut pending)| async move {
            while pending.is_empty() {
                match input.next().await {
                    Some(Ok(bytes)) => {
                        if bytes.len() > limits.blob_bytes {
                            return Err(Error::TooLarge);
                        }
                        pending = bytes;
                    }
                    Some(Err(_)) => return Err(Error::Input),
                    None => return Ok(None),
                }
            }
            let size = pending.len().min(limits.chunk_bytes);
            let chunk = pending.split_to(size).to_vec();
            Ok(Some((chunk, (input, pending))))
        },
    ))
}
pub(crate) fn failure(error: Error) -> Response {
    let (status, category) = match error {
        Error::Unknown | Error::Core(rom::Error::Unknown) => {
            (StatusCode::SERVICE_UNAVAILABLE, "outcome_unknown")
        }
        Error::Core(rom::Error::NotCommitted) => (StatusCode::SERVICE_UNAVAILABLE, "not_committed"),
        Error::Denied | Error::Core(rom::Error::Denied) => (StatusCode::FORBIDDEN, "denied"),
        Error::Conflict | Error::Core(rom::Error::Conflict) => (StatusCode::CONFLICT, "conflict"),
        Error::Missing | Error::Core(rom::Error::Missing) => (StatusCode::NOT_FOUND, "missing"),
        Error::TooLarge | Error::Core(rom::Error::TooLarge) => {
            (StatusCode::PAYLOAD_TOO_LARGE, "too_large")
        }
        Error::Overloaded | Error::Core(rom::Error::Overloaded) => {
            (StatusCode::TOO_MANY_REQUESTS, "overloaded")
        }
        Error::Closed | Error::Core(rom::Error::Closed) => {
            (StatusCode::SERVICE_UNAVAILABLE, "closed")
        }
        Error::Invalid
        | Error::Input
        | Error::Unsupported
        | Error::Core(rom::Error::Invalid { .. })
        | Error::Core(rom::Error::Unsupported(_)) => (StatusCode::BAD_REQUEST, "invalid"),
        Error::Timeout => (StatusCode::REQUEST_TIMEOUT, "timeout"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
    };
    no_store((status, axum::Json(serde_json::json!({"error":category}))).into_response())
}
