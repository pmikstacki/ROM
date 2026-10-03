use super::support::{Scratch, child_process};
use std::{
    collections::BTreeSet,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Barrier},
};

#[test]
fn concurrent_scratch_directories_are_unique_and_private() {
    let start = Arc::new(Barrier::new(16));
    let directories = std::thread::scope(|threads| {
        let handles = (0..16)
            .map(|_| {
                let start = start.clone();
                threads.spawn(move || {
                    start.wait();
                    (0..4).map(|_| Scratch::new()).collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    let paths = directories
        .iter()
        .map(|scratch| scratch.0.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        paths.len(),
        64,
        "concurrent fixtures must own distinct directories"
    );
    for scratch in directories {
        assert_eq!(
            std::fs::metadata(&scratch.0).unwrap().permissions().mode() & 0o777,
            0o700,
            "fixture parent must be private"
        );
    }
    assert!(
        paths.iter().all(|path| !path.exists()),
        "each fixture cleans up only its owned directory"
    );
}

#[test]
fn preexisting_directory_is_preserved_and_creation_retries() {
    let parent = Scratch::new();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--ignored",
            "--exact",
            "scratch::collision_child",
            "--nocapture",
        ])
        .env("ROM_PROVIDER_AUTH_COLLISION_CHILD", "1")
        .env("TMPDIR", &parent.0);
    assert!(child_process::Process::spawn(command).wait().success());
}

#[test]
#[ignore = "private subprocess fixture invoked by preexisting_directory_is_preserved_and_creation_retries"]
fn collision_child() {
    assert!(std::env::var_os("ROM_PROVIDER_AUTH_COLLISION_CHILD").is_some());
    use std::os::unix::fs::DirBuilderExt;
    // The filtered fresh process has not allocated a Scratch sequence number yet.
    let path = std::env::temp_dir().join(format!("rom-host-auth-{}-0", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .unwrap();
    let occupied = Scratch(path);
    let marker = occupied.0.join("existing-marker");
    std::fs::write(&marker, b"preserved fixture").unwrap();
    let fresh = Scratch::new();
    assert_ne!(fresh.0, occupied.0);
    drop(fresh);
    assert_eq!(std::fs::read(&marker).unwrap(), b"preserved fixture");
}
