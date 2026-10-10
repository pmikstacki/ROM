//! Same-runtime characterization of credential-cache waits and core gate pressure.
use super::{
    Cached, Credentials,
    credentials_tests::{Fixture, bind_barrier::Controlled},
};
use crate::{AuthOperation, AuthSnapshot, AuthStage, session::SessionEvidence};
use rom::CoreOverloadStats;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

const BOUND: Duration = Duration::from_secs(2);

struct Blocked {
    auth: AuthSnapshot,
    owned_io: usize,
    available_io: usize,
}

async fn independent_cache(original: &Credentials) -> Arc<Credentials> {
    let cached = original.cached.lock().await;
    // Clone immutable, genuinely verified evidence, never a previously bound Actor.
    Arc::new(Credentials {
        activation: original.activation.clone(),
        token: original.token.clone(),
        nonce: original.nonce.clone(),
        access_token: original.access_token.clone(),
        code: original.code.clone(),
        expiry: original.expiry,
        cached: tokio::sync::Mutex::new(Cached {
            proof: cached.proof.clone(),
            until: cached.until,
        }),
    })
}

async fn characterize(independent: bool, expected_io: usize) {
    let storage = Arc::new(Controlled::new(Arc::new(
        rom_sqlite::Sqlite::open(":memory:").unwrap(),
    )));
    let fixture = Fixture::with_diagnostics(200, false, storage.clone(), None, true).await;
    let shared = fixture.host.shared.clone();
    let session = shared.sessions.lookup(&fixture.cookie, 100).unwrap();
    let mut cookies = Vec::new();
    for _ in 0..8 {
        if independent {
            let credentials = independent_cache(&fixture.credentials).await;
            let session = shared
                .sessions
                .insert(
                    SessionEvidence {
                        actor: session.evidence.actor.clone(),
                        user_id: session.evidence.user_id.clone(),
                        token_expiry: session.evidence.token_expiry,
                        credentials: Some(credentials),
                    },
                    100,
                )
                .unwrap();
            cookies.push(session.cookie().to_owned());
        } else {
            cookies.push(fixture.cookie.clone());
        }
    }
    let baseline_io = shared.runtime.available_io_capacity();
    let expected_available = baseline_io.checked_sub(expected_io).unwrap();
    storage.arm();
    let tasks: Vec<_> = cookies
        .into_iter()
        .map(|cookie| {
            let shared = shared.clone();
            tokio::spawn(async move { crate::authentication::resolve(&shared, &cookie).await })
        })
        .collect();
    let entered = tokio::time::timeout(BOUND, storage.started.notified()).await;
    let blocked = if entered.is_ok() {
        tokio::time::timeout(BOUND, async {
            loop {
                let auth = shared
                    .auth
                    .authentication_diagnostics()?
                    .ok_or(rom::Error::Closed)?;
                let status = shared.runtime.status()?;
                if auth.operation(AuthOperation::Session).admitted == 8
                    && auth.stage(AuthStage::CredentialWait).succeeded == expected_io as u64
                    && status.owned_work == expected_io
                    && status.available_io_permits == expected_available
                {
                    break Ok::<_, rom::Error>(Blocked {
                        auth,
                        owned_io: status.owned_work,
                        available_io: status.available_io_permits,
                    });
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .ok()
        .and_then(Result::ok)
    } else {
        None
    };
    // Both observation bounds together are shorter than the storage barrier's 5s limit.
    // Release and join before checking any result, including a failed observation.
    storage.release();
    let joined = tokio::time::timeout(BOUND, futures_util::future::join_all(tasks)).await;
    let final_auth = shared.auth.authentication_diagnostics();
    let final_core = shared.runtime.core_overload_stats();
    let final_status = shared.runtime.status();
    let available_io = shared.runtime.available_io_capacity();
    let key_requests = fixture.requests.load(Ordering::SeqCst);
    let original_expiry = fixture.credentials.expiry;
    let session_expiry = session.expires_at();
    let auth_capacity = shared.config.limits.authentication_jobs;
    let auth_timeout = shared.config.limits.acquisition_timeout;
    fixture.close().await;

    assert!(
        entered.is_ok(),
        "genuine bind must reach the native user-read barrier"
    );
    let blocked =
        blocked.expect("all eight accepted callbacks must reach their actual waiting boundary");
    let final_auth = final_auth.unwrap().unwrap();
    let outstanding_io = final_status.unwrap().owned_work;
    let results = joined.expect("released callbacks must physically complete");
    assert_eq!(results.len(), 8);
    for result in results {
        assert_eq!(result.unwrap().unwrap().valid_until(), Some(130));
    }
    assert_eq!(blocked.owned_io, expected_io);
    assert_eq!(blocked.available_io, baseline_io - expected_io);
    assert_eq!(
        blocked.auth.stage(AuthStage::CredentialWait).succeeded,
        expected_io as u64
    );
    assert_eq!(
        blocked.auth.stage(AuthStage::CachedBind).succeeded,
        0,
        "the first native read holds the shared Runtime gate for every cache"
    );
    assert_eq!(blocked.auth.operation(AuthOperation::Session).completed, 0);
    let counts = final_auth.operation(AuthOperation::Session);
    assert_eq!(
        (counts.admitted, counts.completed, counts.released),
        (8, 8, 8)
    );
    assert_eq!(
        (
            counts.rejected_capacity,
            counts.downstream_overloaded,
            counts.outstanding
        ),
        (0, 0, 0)
    );
    assert!(!final_auth.overflowed);
    assert_eq!(final_core, CoreOverloadStats::default());
    assert_eq!(outstanding_io, 0);
    assert_eq!(available_io, baseline_io);
    assert_eq!(key_requests, 0, "live original proof needs no renewal");
    assert_eq!((original_expiry, session_expiry), (400, 400));
    assert_eq!(auth_capacity, 8);
    assert_eq!(auth_timeout, Duration::from_secs(5));
    println!(
        "{}",
        serde_json::json!({"schema":"rom-auth-cache-gate-characterization-v1",
        "case":if independent {"independent-caches"} else {"shared-cache"},
        "callers":8,"credential_wait_completed":expected_io,"owned_io_while_blocked":expected_io,
        "authoritative_binds_completed_while_blocked":0,"acceptance":false})
    );
}

#[tokio::test]
async fn eight_shared_cache_callers_hold_one_core_io_while_the_native_gate_is_blocked() {
    characterize(false, 1).await;
}

#[tokio::test]
async fn eight_independent_caches_hold_eight_core_io_while_the_same_native_gate_is_blocked() {
    characterize(true, 8).await;
}
