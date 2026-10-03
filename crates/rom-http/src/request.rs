use crate::{error::Failure, json, server::Shared};
use axum::{body::to_bytes, extract::Request};
use rom::{Actor, Error};

pub(super) async fn decode<T: serde::de::DeserializeOwned>(
    shared: &Shared,
    request: Request,
) -> Result<(Actor, T, tokio::sync::OwnedSemaphorePermit), Failure> {
    if *shared.closed.borrow() {
        return Err(Error::Closed.into());
    }
    let permit = shared.bodies.clone().try_acquire_owned().map_err(|e| {
        Failure(match e {
            tokio::sync::TryAcquireError::Closed => Error::Closed,
            _ => Error::Overloaded,
        })
    })?;
    let actor = (shared.auth)(request.headers())?;
    let bytes = tokio::time::timeout(
        shared.limits.body_timeout,
        to_bytes(request.into_body(), shared.limits.body_bytes),
    )
    .await
    .map_err(|_| Failure(Error::Overloaded))?
    .map_err(|_| Failure(Error::TooLarge))?;
    Ok((actor, json::parse(&bytes)?, permit))
}
