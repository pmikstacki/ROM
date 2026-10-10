use super::validate_private_path;
use std::path::Path;
#[test]
fn generated_nonce_directory_is_accepted_but_neighbors_and_traversal_are_denied() {
    let prefix = "/var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-0123456789abcdef01234567/";
    for name in [
        "source",
        "destination",
        "backup",
        "source-stop",
        "destination-stop",
    ] {
        assert!(validate_private_path(Path::new(&format!("{prefix}{name}"))).is_ok());
    }
    for suffix in ["../source", "source/../database", "other", "source-alias"] {
        assert!(validate_private_path(Path::new(&format!("{prefix}{suffix}"))).is_err());
    }
    assert!(validate_private_path(Path::new("/var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-unowned/source")).is_err());
}
