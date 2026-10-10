//! Cancellable observation waits over supervised current identity checks.
use crate::{
    AuthOperation,
    router::Shared,
    session::{Session, SessionValidity},
};
use rom::Actor;
use std::sync::Arc;
pub(crate) enum Gate {
    Ready,
    Cancelled,
    Terminal(&'static str),
}
fn terminal(shared: &Shared, session: &Session, actor: &Actor) -> Option<Gate> {
    let now = shared.config.clock.now();
    match session.validity(now) {
        SessionValidity::Cancelled => Some(Gate::Cancelled),
        SessionValidity::Expired => Some(Gate::Terminal("identity_expired")),
        SessionValidity::Current if actor.valid_until().is_none_or(|end| now >= end) => {
            Some(Gate::Terminal("identity_expired"))
        }
        SessionValidity::Current => None,
    }
}
pub(crate) async fn check(shared: &Arc<Shared>, session: &Arc<Session>, actor: &Actor) -> Gate {
    let mut cancelled = session.cancellation();
    if let Some(gate) = terminal(shared, session, actor) {
        return gate;
    }
    let Some(credentials) = session.evidence.credentials.clone() else {
        return Gate::Terminal("denied");
    };
    let owned_shared = shared.clone();
    // Dropping this waiter leaves the accepted bind and its permit owned by Supervisor.
    let current = shared.auth.run_queued_tagged(
        AuthOperation::CurrentStream,
        shared.config.limits.acquisition_timeout,
        async move { credentials.current(&owned_shared).await },
    );
    tokio::pin!(current);
    loop {
        if let Some(gate) = terminal(shared, session, actor) {
            return gate;
        }
        tokio::select! {biased;
            _=cancelled.changed()=>return Gate::Cancelled,
            result=&mut current=>{
                if let Some(gate)=terminal(shared,session,actor) {return gate;}
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
