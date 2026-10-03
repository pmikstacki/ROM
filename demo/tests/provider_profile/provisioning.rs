use super::support::*;
use rom::Error;
use rom_demo::provider_profile::{LocalMode, build, provision};
use rom_identity::IdentityProvider;
use std::sync::Arc;
#[tokio::test]
async fn both_store_resume_preserves_revisions_and_changed_input_is_rejected() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let path = scratch.0.join("db");
        let config = settings("http://127.0.0.1:1/introspect");
        let runtime = build(
            storage(redb, &path),
            Arc::new(Time),
            &config,
            LocalMode::Provisioning,
        )
        .unwrap();
        provision(&runtime, &config).await.unwrap();
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let runtime = build(
            storage(redb, &path),
            Arc::new(Time),
            &config,
            LocalMode::Provisioning,
        )
        .unwrap();
        provision(&runtime, &config).await.unwrap();
        let row = runtime
            .read::<IdentityProvider>(
                &rom_demo::provider_profile::configuration_reader(),
                "provider",
            )
            .await
            .unwrap();
        assert_eq!(row.revision, 1);
        let mut changed = config.clone();
        changed.provider.issuer = "https://changed.invalid".into();
        assert!(matches!(
            provision(&runtime, &changed).await,
            Err(Error::IdentityMismatch)
        ));
        runtime.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn each_lost_ack_step_resumes_after_native_close_and_reopen() {
    for redb in [false, true] {
        for step in 1..=3 {
            let scratch = Scratch::new();
            let path = scratch.0.join("db");
            let config = settings("http://127.0.0.1:1/introspect");
            let db = Db::open(redb, &path);
            let wrapped = Arc::new(LostAck {
                base: db.storage(),
                remaining: step.into(),
            });
            let runtime = build(
                wrapped.clone(),
                Arc::new(Time),
                &config,
                LocalMode::Provisioning,
            )
            .unwrap();
            assert!(matches!(
                provision(&runtime, &config).await,
                Err(Error::Unknown)
            ));
            assert_eq!(db.counts(), [step as u64, step as u64, step as u64, 0]);
            runtime.shutdown().await.unwrap();
            drop(runtime);
            drop(wrapped);
            drop(db);
            let db = Db::open(redb, &path);
            let runtime = build(
                db.storage(),
                Arc::new(Time),
                &config,
                LocalMode::Provisioning,
            )
            .unwrap();
            provision(&runtime, &config).await.unwrap();
            assert_eq!(db.counts(), [3, 3, 3, 0]);
            provision(&runtime, &config).await.unwrap();
            assert_eq!(db.counts(), [3, 3, 3, 0]);
            let mut changed = config.clone();
            changed.user.display_name = "Changed".into();
            assert!(matches!(
                provision(&runtime, &changed).await,
                Err(Error::IdentityMismatch)
            ));
            assert_eq!(db.counts(), [3, 3, 3, 0]);
            runtime.shutdown().await.unwrap();
        }
    }
}
