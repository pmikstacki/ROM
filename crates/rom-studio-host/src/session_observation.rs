use crate::{router::Shared, session::Session};
use axum::{
    body::{Body, BodyDataStream, Bytes},
    response::Response,
};
use futures_util::{StreamExt, stream};
use rom::Actor;
use std::{convert::Infallible, sync::Arc};

struct Observation {
    body: BodyDataStream,
    session: Arc<Session>,
    shared: Arc<Shared>,
    actor: Actor,
}
/// Wrap the existing observer. Its captured Actor is never replaced or extended.
pub(crate) fn guard(
    response: Response,
    shared: Arc<Shared>,
    session: Arc<Session>,
    actor: Actor,
) -> Response {
    let (parts, body) = response.into_parts();
    let observed = stream::unfold(
        Some(Observation {
            body: body.into_data_stream(),
            session,
            shared,
            actor,
        }),
        |state| async move {
            let mut state = state?;
            loop {
                if *state.session.cancellation().borrow() {
                    return None;
                }
                if state
                    .actor
                    .valid_until()
                    .is_none_or(|end| state.shared.config.clock.now() >= end)
                {
                    return terminal("identity_expired");
                }
                let Some(credentials) = &state.session.evidence.credentials else {
                    return terminal("denied");
                };
                if credentials.current(&state.shared).await.is_err() {
                    return terminal("denied");
                }
                let mut cancelled = state.session.cancellation();
                let chunk = tokio::select! {biased;
                    _=cancelled.changed()=>return None,
                    chunk=state.body.next()=>chunk,
                    _=tokio::time::sleep(state.shared.config.limits.observation_poll)=>continue,
                };
                if *cancelled.borrow() {
                    return None;
                }
                if state
                    .actor
                    .valid_until()
                    .is_none_or(|end| state.shared.config.clock.now() >= end)
                {
                    return terminal("identity_expired");
                }
                if credentials.current(&state.shared).await.is_err() {
                    return terminal("denied");
                }
                match chunk {
                    Some(Ok(bytes)) => return Some((Ok::<_, Infallible>(bytes), Some(state))),
                    Some(Err(_)) => return terminal("internal"),
                    None => return None,
                }
            }
        },
    );
    Response::from_parts(parts, Body::from_stream(observed))
}
fn terminal(
    category: &str,
) -> Option<(std::result::Result<Bytes, Infallible>, Option<Observation>)> {
    Some((
        Ok(Bytes::from(format!("event: error\ndata: {category}\n\n"))),
        None,
    ))
}
