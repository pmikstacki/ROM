use super::support::*;
use rom_demo::provider_profile::{
    AuthLimits, LocalMode, build, configuration_reader, provision, serve,
};
use std::{sync::Arc, time::Duration};
#[tokio::test]
async fn stop_drains_paused_authentication_before_closing_runtime() {
    for redb in [false, true] {
        let mut provider = Provider::new(true).await;
        let scratch = Scratch::new();
        let config = settings(&provider.endpoint);
        let runtime = build(
            storage(redb, &scratch.0.join("db")),
            Arc::new(Time),
            &config,
            LocalMode::Provisioning,
        )
        .unwrap();
        provision(&runtime, &config).await.unwrap();
        let auth = authentication(
            &runtime,
            &config,
            &scratch,
            AuthLimits {
                jobs: 1,
                response_timeout: Duration::from_secs(2),
            },
            b"credential",
        );
        let resolver = auth.resolver();
        let caller = tokio::spawn(async move { resolver(headers("Bearer token")).await });
        provider.entered().await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let serving = tokio::spawn(serve(runtime.clone(), auth.clone(), listener, async move {
            stopped.await.unwrap()
        }));
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if matches!(
                    auth.resolver()(headers("Bearer another")).await,
                    Err(rom::Error::Closed)
                ) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(!serving.is_finished());
        runtime
            .read::<rom_identity::IdentityProvider>(&configuration_reader(), "provider")
            .await
            .unwrap();
        assert!(matches!(
            auth.resolver()(headers("Bearer another")).await,
            Err(rom::Error::Closed)
        ));
        provider.release();
        let _ = caller.await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), serving)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(
            runtime
                .read::<rom_identity::IdentityProvider>(&configuration_reader(), "provider")
                .await,
            Err(rom::Error::Closed)
        ));
    }
}
