use crate::{error::category, server::Shared};
use axum::response::{IntoResponse, Response, Sse, sse::Event};
use futures_util::stream;
use rom::Actor;
use std::{convert::Infallible, future::Future, sync::Arc};

type PendingObservation<H, T> = std::pin::Pin<Box<dyn Future<Output = (H, rom::Result<T>)> + Send>>;

fn pending<H, T, F>(mut handle: H, next: Arc<F>) -> PendingObservation<H, T>
where
    H: Send + 'static,
    T: Send + 'static,
    F: for<'a> Fn(&'a mut H) -> std::pin::Pin<Box<dyn Future<Output = rom::Result<T>> + Send + 'a>>
        + Send
        + Sync
        + 'static,
{
    Box::pin(async move {
        let result = next(&mut handle).await;
        (handle, result)
    })
}

pub(super) fn observe<H, T, F>(s: Shared, actor: Actor, handle: H, next: F) -> Response
where
    H: Send + 'static,
    T: serde::Serialize + Send + 'static,
    F: for<'a> Fn(&'a mut H) -> std::pin::Pin<Box<dyn Future<Output = rom::Result<T>> + Send + 'a>>
        + Send
        + Sync
        + 'static,
{
    let closed = s.closed.subscribe();
    let next = Arc::new(next);
    let events = stream::unfold(
        Some((pending(handle, next.clone()), closed, next)),
        move |state| {
            let poll = s.limits.observation_poll;
            let runtime = s.runtime.clone();
            let actor = actor.clone();
            async move {
                let (mut pending_read, mut closed, next) = state?;
                if *closed.borrow() {
                    return None;
                }
                if let Err(error) = runtime.observation_status(&actor) {
                    return Some((
                        Ok::<_, Infallible>(
                            Event::default().event("error").data(category(&error).0),
                        ),
                        None,
                    ));
                }
                let result = tokio::select! {
                    _=closed.changed()=>return None,
                    result=tokio::time::timeout(poll,&mut pending_read)=>result,
                };
                if let Err(error) = runtime.observation_status(&actor) {
                    return Some((
                        Ok(Event::default().event("error").data(category(&error).0)),
                        None,
                    ));
                }
                match result {
                    Err(_) => Some((
                        Ok(Event::default().comment("keepalive")),
                        Some((pending_read, closed, next)),
                    )),
                    Ok((handle, Ok(value))) => {
                        match Event::default().event("data").json_data(value) {
                            Ok(event) => Some((
                                Ok(event),
                                Some((pending(handle, next.clone()), closed, next)),
                            )),
                            Err(_) => {
                                Some((Ok(Event::default().event("error").data("internal")), None))
                            }
                        }
                    }
                    Ok((_, Err(error))) => Some((
                        Ok(Event::default().event("error").data(category(&error).0)),
                        None,
                    )),
                }
            }
        },
    );
    Sse::new(events).into_response()
}
