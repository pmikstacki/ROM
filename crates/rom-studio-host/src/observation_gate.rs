//! Cancellable observation waits over supervised current identity checks.
use crate::{router::Shared, session::Session};
use rom::Actor;
use std::sync::Arc;
pub(crate) enum Gate {
    Ready,
    Cancelled,
    Terminal(&'static str),
}
fn expired(shared: &Shared, actor: &Actor) -> bool {
    actor
        .valid_until()
        .is_none_or(|end| shared.config.clock.now() >= end)
}
pub(crate) async fn check(shared: &Arc<Shared>, session: &Arc<Session>, actor: &Actor) -> Gate {
    let mut cancelled = session.cancellation();
    if *cancelled.borrow() {
        return Gate::Cancelled;
    }
    if expired(shared, actor) {
        return Gate::Terminal("identity_expired");
    }
    let Some(credentials) = session.evidence.credentials.clone() else {
        return Gate::Terminal("denied");
    };
    let owned_shared = shared.clone();
    // Dropping this waiter leaves the accepted bind and its permit owned by Supervisor.
    let current = shared
        .auth
        .run(async move { credentials.current(&owned_shared).await });
    tokio::pin!(current);
    loop {
        if *cancelled.borrow() {
            return Gate::Cancelled;
        }
        if expired(shared, actor) {
            return Gate::Terminal("identity_expired");
        }
        tokio::select! {biased;
            _=cancelled.changed()=>return Gate::Cancelled,
            result=&mut current=>{
                if *cancelled.borrow() {return Gate::Cancelled;}
                if expired(shared,actor) {return Gate::Terminal("identity_expired");}
                return match result {
                    Ok(_)=>Gate::Ready,
                    Err(rom::Error::Denied)=>Gate::Terminal("denied"),
                    Err(rom::Error::Overloaded)=>Gate::Terminal("overloaded"),
                    Err(rom::Error::Closed)=>Gate::Terminal("closed"),
                    Err(_)=>Gate::Terminal("internal"),
                };
            },
            _=tokio::time::sleep(shared.config.limits.observation_poll)=>{},
        }
    }
}
