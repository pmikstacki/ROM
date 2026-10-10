use crate::provisioning;
use rom_studio_host::{HostConfig, OidcProviderConfig, StudioHost};
use std::time::{Duration, Instant};
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("private host configuration required")?;
    let configuration =
        provisioning::read(&path).inspect_err(|_| eprintln!("host_fixture_stage=configuration"))?;
    let (runtime, actor) = provisioning::runtime(&configuration)
        .inspect_err(|_| eprintln!("host_fixture_stage=runtime"))?;
    provisioning::seed(&runtime, &actor, &configuration)
        .await
        .inspect_err(|_| eprintln!("host_fixture_stage=seed"))?;
    let mut mailbox = configuration
        .control_directory
        .as_deref()
        .map(crate::control::Mailbox::new)
        .transpose()?;
    let control_runtime = runtime.clone();
    let control_actor = actor.clone();
    let control_subject = configuration.verified_synthetic_subject.clone();
    let control_failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let failed = control_failed.clone();
    let origin = provisioning::approved_origin(&configuration)?.to_owned();
    let profile = provisioning::transport_profile(&configuration)?;
    let backchannel = match (
        profile,
        &configuration.private_token_endpoint,
        &configuration.private_jwks_endpoint,
    ) {
        (provisioning::TransportProfile::TerminatedLoopback, Some(token), Some(jwks)) => {
            Some(rom_studio_host::TrustedLoopbackBackchannel::new(
                &configuration.issuer,
                token,
                jwks,
            ))
        }
        _ => None,
    };
    let mut config = HostConfig::new(
        &origin,
        "/rom-studio/",
        &configuration.assets_directory,
        actor,
    )
    .allow_loopback_http(origin == "http://127.0.0.1:44391")
    .settings("main")
    .provider(OidcProviderConfig {
        authority: "authentik".into(),
        label: "Synthetic Authentik".into(),
        issuer: configuration.issuer,
        client_id: configuration.client_id,
        authorization_endpoint: configuration.authorization_endpoint,
        token_endpoint: configuration.token_endpoint,
        jwks_endpoint: configuration.jwks_endpoint,
        client_secret: Some(configuration.client_secret),
    });
    if let Some(backchannel) = backchannel {
        config = config.provider_backchannel("authentik", backchannel);
    }
    config.limits.sessions = 4;
    config.limits.login_attempts = 4;
    config.limits.authentication_jobs = 2;
    config.limits.acquisition_timeout = Duration::from_secs(3);
    config.limits.acquisition_bytes = 65_536;
    config.limits.assets = 2;
    config.limits.asset_bytes = 4096;
    let host = StudioHost::new(runtime, config)
        .inspect_err(|_| eprintln!("host_fixture_stage=host-construction"))?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:44391")
        .await
        .inspect_err(|_| eprintln!("host_fixture_stage=listener"))?;
    let lane = if profile == provisioning::TransportProfile::NativeTls {
        "native-tls-source-authoring"
    } else {
        "source-authoring-reproduction"
    };
    // Readiness establishes no handshake. Only the runner can record TLS verification.
    println!("{{\"status\":\"ready\",\"lane\":\"{lane}\",\"tls_verified\":false}}");
    host.serve(listener, async move {
        let deadline = Instant::now() + Duration::from_secs(120);
        while Instant::now() < deadline && !std::path::Path::new(&configuration.stop_file).exists()
        {
            if let Some(mailbox) = &mut mailbox
                && mailbox
                    .poll(&control_runtime, &control_actor, &control_subject)
                    .await
                    .is_err()
            {
                failed.store(true, std::sync::atomic::Ordering::SeqCst);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await?;
    if control_failed.load(std::sync::atomic::Ordering::SeqCst) {
        eprintln!("host_fixture_stage=private-control");
        return Err("private fixture control failed or had an unacknowledged outcome".into());
    }
    println!("{{\"status\":\"drained\"}}");
    Ok(())
}
