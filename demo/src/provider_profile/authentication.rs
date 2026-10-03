//! Host-owned verification jobs; caller cancellation never aborts accepted work.
use super::{ApprovedProvider, AuthLimits, SecretFiles, lifecycle::Jobs, verification};
use rom::{Actor, Clock, Error, Result, Runtime};
use rom_auth::introspection::EndpointPolicy;
use rom_http::{AsyncAuthResolver, HeaderMap};
use rom_identity::ProviderActivation;
use std::sync::Arc;
use tokio::sync::oneshot;

struct Shared {
    runtime: Runtime,
    reader: Actor,
    clock: Arc<dyn Clock>,
    provider: ApprovedProvider,
    secrets: SecretFiles,
    policy: EndpointPolicy,
    limits: AuthLimits,
    jobs: Arc<Jobs>,
}
/// Cloneable authentication lifecycle. Close intake and drain before Runtime shutdown.
/// Accepted jobs retain capacity and Runtime ownership after a caller times out or
/// disconnects. No blocking HTTP verifier is constructed on an async runtime worker.
#[derive(Clone)]
pub struct HostAuth {
    shared: Arc<Shared>,
}
impl HostAuth {
    /// Construct from explicit trusted host inputs. The same clock must also be
    /// passed to `Runtime::builder().clock`; credentials are acquired only per job.
    #[allow(clippy::too_many_arguments)] // Exact composition seam specified by the profile.
    pub fn new(
        runtime: Runtime,
        reader: Actor,
        clock: Arc<dyn Clock>,
        provider: ApprovedProvider,
        secrets: SecretFiles,
        policy: EndpointPolicy,
        limits: AuthLimits,
    ) -> Result<Self> {
        limits.validate()?;
        provider.validate()?;
        Ok(Self {
            shared: Arc::new(Shared {
                runtime,
                reader,
                clock,
                provider,
                secrets,
                policy,
                limits,
                jobs: Jobs::new(limits.jobs),
            }),
        })
    }
    pub fn resolver(&self) -> AsyncAuthResolver {
        let shared = self.shared.clone();
        Arc::new(move |headers| {
            let shared = shared.clone();
            Box::pin(async move {
                shared.jobs.check_open()?;
                let token = bearer(headers)?;
                let job = shared.jobs.admit()?;
                let timeout = shared.limits.response_timeout;
                let (sender, receiver) = oneshot::channel();
                // The job and its completion condition outlive this caller future.
                tokio::spawn(async move {
                    let result = authenticate(&shared, token).await;
                    drop(shared);
                    drop(job);
                    let _ = sender.send(result);
                });
                tokio::time::timeout(timeout, receiver)
                    .await
                    .map_err(|_| Error::Overloaded)?
                    .map_err(|_| Error::Denied)?
            })
        })
    }
    /// Idempotently reject new jobs. Already accepted verification remains owned.
    pub fn close(&self) {
        self.shared.jobs.close();
    }
    /// Await accepted jobs. Canceling a waiter preserves tracking for another call.
    /// Call `close` first to prevent new admission while draining.
    pub async fn drain(&self) -> Result<()> {
        self.shared.jobs.drain().await
    }
}
fn bearer(headers: HeaderMap) -> Result<String> {
    let mut values = headers.get_all("authorization").iter();
    let value = values.next().ok_or(Error::Denied)?;
    if values.next().is_some() || value.as_bytes().len() > 4103 {
        return Err(Error::Denied);
    }
    let token = value
        .to_str()
        .map_err(|_| Error::Denied)?
        .strip_prefix("Bearer ")
        .ok_or(Error::Denied)?;
    if token.is_empty()
        || token.len() > 4096
        || token
            .bytes()
            .any(|v| v.is_ascii_whitespace() || v.is_ascii_control())
    {
        return Err(Error::Denied);
    }
    Ok(token.to_owned())
}
async fn authenticate(shared: &Shared, token: String) -> Result<Actor> {
    let activation =
        ProviderActivation::read(&shared.runtime, &shared.reader, &shared.provider.authority)
            .await
            .map_err(|_| Error::Denied)?;
    let provider = shared.provider.clone();
    let secrets = shared.secrets.clone();
    let policy = shared.policy;
    let clock = shared.clock.clone();
    let identity = tokio::task::spawn_blocking(move || {
        verification::verify(activation, provider, secrets, policy, clock, token)
    })
    .await
    .map_err(|_| Error::Denied)??;
    identity
        .bind(&shared.runtime)
        .await
        .map_err(|_| Error::Denied)
}
