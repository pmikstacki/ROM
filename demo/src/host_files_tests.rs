//! Executed opened-file admission cases. These tests do not simulate a pathname race.
#[cfg(target_os = "linux")]
mod linux {
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};
    fn root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "rom-host-file-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir(&path).expect("fixture");
        path
    }
    #[test]
    fn private_reader_rejects_link_public_permissions_bounds_and_invalid_utf8() {
        let root = root();
        let private = root.join("private");
        let alias = root.join("link");
        std::fs::write(&private, "private").expect("synthetic bytes");
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o600)).expect("mode");
        #[cfg(feature = "provider-profile")]
        assert_eq!(
            crate::host_files::read_private(&private, 32).expect("admission"),
            "private"
        );
        #[cfg(feature = "studio")]
        assert_eq!(
            crate::host_files::read_owned_private(&private, 32).expect("admission"),
            "private"
        );
        std::os::unix::fs::symlink(&private, &alias).expect("link");
        #[cfg(feature = "provider-profile")]
        assert!(crate::host_files::read_private(&alias, 32).is_err());
        #[cfg(feature = "studio")]
        assert!(crate::host_files::read_owned_private(&alias, 32).is_err());
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o644)).expect("mode");
        #[cfg(feature = "studio")]
        assert!(crate::host_files::read_owned_private(&private, 32).is_err());
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o600)).expect("mode");
        std::fs::write(&private, vec![b'x'; 33]).expect("synthetic oversized bytes");
        #[cfg(feature = "studio")]
        assert!(crate::host_files::read_owned_private(&private, 32).is_err());
        std::fs::write(&private, [0xff]).expect("synthetic invalid UTF8");
        #[cfg(feature = "studio")]
        assert_eq!(
            crate::host_files::read_owned_private(&private, 32)
                .expect_err("reject")
                .to_string(),
            "Denied"
        );
        std::fs::remove_dir_all(root).expect("remove synthetic fixture");
    }

    #[cfg(feature = "studio")]
    #[test]
    fn regular_reader_accepts_root_owned_read_only_profile_and_rejects_writable_profile() {
        let root = root();
        let profile = root.join("profile.json");
        std::fs::write(&profile, "trusted profile").expect("profile");
        std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o444))
            .expect("read-only mode");
        assert_eq!(
            crate::host_files::read_regular(&profile, 32).expect("root-owned profile is trusted"),
            "trusted profile"
        );
        std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o664))
            .expect("writable owner mode");
        assert!(crate::host_files::read_regular(&profile, 32).is_err());
        std::fs::remove_dir_all(root).expect("remove synthetic fixture");
    }

    #[cfg(feature = "studio")]
    #[test]
    fn systemd_credential_reader_accepts_private_manager_file_only_in_credential_directory() {
        let root = root();
        let credential = root.join("client-secret");
        let outside = root.join("outside-secret");
        std::fs::write(&credential, "credential").expect("credential");
        std::fs::set_permissions(&credential, std::fs::Permissions::from_mode(0o440))
            .expect("systemd credential mode");
        assert_eq!(
            crate::host_files::read_systemd_credential(&credential, 32, &root)
                .expect("systemd credential is admitted"),
            "credential"
        );

        std::fs::write(&outside, "outside").expect("outside credential");
        std::fs::set_permissions(&outside, std::fs::Permissions::from_mode(0o440))
            .expect("systemd credential mode");
        assert!(
            crate::host_files::read_systemd_credential(&outside, 32, &root.join("other")).is_err()
        );

        std::fs::set_permissions(&credential, std::fs::Permissions::from_mode(0o600))
            .expect("writable credential mode");
        assert!(crate::host_files::read_systemd_credential(&credential, 32, &root).is_err());
        std::fs::remove_dir_all(root).expect("remove synthetic fixture");
    }

    #[test]
    fn opened_fifo_is_rejected_without_waiting_for_a_writer() {
        let root = root();
        let fifo = root.join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .expect("mkfifo")
                .success()
        );
        let (send, receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            #[cfg(feature = "studio")]
            let result = crate::host_files::read_regular(&fifo, 32);
            #[cfg(all(not(feature = "studio"), feature = "provider-profile"))]
            let result = crate::host_files::read_private(&fifo, 32);
            send.send(result.is_err()).expect("report admission");
        });
        assert!(
            receive
                .recv_timeout(std::time::Duration::from_secs(2))
                .expect("FIFO reader must reject without a writer")
        );
        worker.join().expect("reader returned");
        std::fs::remove_dir_all(root).expect("remove synthetic fixture");
    }
}
