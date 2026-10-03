use super::support::*;
use rom_demo::provider_profile::AuthLimits;

#[tokio::test]
async fn private_regular_file_verifies_without_trimming_secret_material() {
    let scratch = Scratch::new();
    let provider =
        Provider::expecting_authorization("Basic aW50cm9zcGVjdG9yOmV4YWN0LXNlY3JldCUwQQ==").await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(
        scratch.file("v1", b"exact-secret\n", 0o600),
        AuthLimits::default(),
    );
    let actor = auth.resolver()(headers("Bearer fixture-token"))
        .await
        .unwrap();
    assert_eq!(actor.subject, "service");
    assert_eq!(actor.principal_kind(), rom::PrincipalKind::Service);
    assert_eq!(actor.valid_until(), Some(NOW + 5));
    assert!(actor.host_stamp().is_some());
    assert_eq!(provider.count(), 1);
    auth.close();
    auth.drain().await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn invalid_files_deny_before_contact_and_errors_disclose_no_path_or_material() {
    let scratch = Scratch::new();
    let provider = Provider::new(false).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let private = scratch.file("good", b"secret-marker", 0o600);
    let link = scratch.0.join("symlink");
    std::os::unix::fs::symlink(&private, &link).unwrap();
    let files = vec![
        scratch.file("empty", b"", 0o600),
        scratch.file("large", &vec![b'x'; 4097], 0o600),
        scratch.file("utf8", &[255], 0o600),
        scratch.file("public", b"secret-marker", 0o644),
        link,
        scratch.0.clone(),
        scratch.0.join("missing"),
    ];
    for file in files {
        let auth = fixture.auth(file, AuthLimits::default());
        let error = auth.resolver()(headers("Bearer token-marker"))
            .await
            .unwrap_err();
        assert!(matches!(error, rom::Error::Denied));
        let diagnostic = format!("{error:?} {error}");
        for marker in ["secret-marker", "token-marker", scratch.0.to_str().unwrap()] {
            assert!(!diagnostic.contains(marker));
        }
        auth.close();
        auth.drain().await.unwrap();
    }
    assert_eq!(provider.count(), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn exact_secret_and_bearer_byte_bounds_are_admitted() {
    let scratch = Scratch::new();
    let provider = Provider::new(false).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(
        scratch.file("v1", &vec![b'x'; 4096], 0o600),
        AuthLimits::default(),
    );
    let actor = auth.resolver()(headers(&format!("Bearer {}", "x".repeat(4096))))
        .await
        .unwrap();
    assert_eq!(actor.subject, "service");
    assert_eq!(provider.count(), 1);
    auth.close();
    auth.drain().await.unwrap();
    fixture.finish().await;
}

#[test]
fn fifo_without_writer_denies_within_bound() {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", "files::fifo_child", "--nocapture"])
        .env("ROM_PROVIDER_AUTH_FIFO_CHILD", "1");
    assert!(child_process::Process::spawn(command).wait().success());
}

#[tokio::test]
#[ignore = "private subprocess fixture invoked by fifo_without_writer_denies_within_bound"]
async fn fifo_child() {
    assert!(std::env::var_os("ROM_PROVIDER_AUTH_FIFO_CHILD").is_some());
    let scratch = Scratch::new();
    let fifo = scratch.0.join("fifo");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let provider = Provider::new(false).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(fifo, AuthLimits::default());
    tokio::time::timeout(std::time::Duration::from_secs(1), denied(&auth))
        .await
        .unwrap();
    assert_eq!(provider.count(), 0);
    auth.close();
    auth.drain().await.unwrap();
    fixture.finish().await;
}
