use rom_studio_host::HostConfig;
#[test]
fn root_mount_is_valid_without_an_empty_slice_panic() {
    assert!(
        HostConfig::new(
            "https://studio.example",
            "/",
            "/assets",
            rom::Actor::trusted("host", "configuration")
        )
        .validate()
        .is_ok()
    );
}
