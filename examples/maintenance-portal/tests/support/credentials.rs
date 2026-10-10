//! Explicit fixed credentials for private loopback fixtures, never production identity.
use rom::{Actor, PrincipalKind};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) fn resolver(enabled: Arc<AtomicBool>) -> rom_http::AuthResolver {
    Arc::new(move |headers| {
        let authorization = headers.get("authorization");
        if authorization.is_none() {
            return Ok(rom_maintenance_portal::public_guest());
        }
        let subject = match authorization.and_then(|value| value.to_str().ok()) {
            Some("Bearer fixture-alice") if enabled.load(Ordering::SeqCst) => "alice",
            Some("Bearer fixture-bob") => "bob",
            _ => return Err(rom::Error::Denied),
        };
        Ok(Actor::trusted("maintenance-portal", subject).with_kind(PrincipalKind::Human))
    })
}
