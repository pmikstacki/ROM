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
