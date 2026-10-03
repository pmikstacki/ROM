use super::*;
use crate::{NativeAccess, NativeOwnership};
use std::{
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, symlink},
    },
    process::{Child, Command},
    time::{Duration, Instant},
};

// A concurrent fork can retain another test's flock until the child executes.
// Keep process spawning separate from this module's immediate unlock assertions.
static PROCESS_BOUNDARY: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-native-publication-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn stage(&self) -> Stage {
        let stage = Stage::new(&self.path("database")).unwrap();
        fs::write(stage.path(), b"validated native bytes").unwrap();
        stage
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_finalization_removes_only_the_private_alias_and_keeps_reservation() {
    let _process_boundary = PROCESS_BOUNDARY.lock().unwrap();
    let directory = Directory::new();
    let destination = directory.path("database");
    let owner = NativeOwnership::acquire(&destination, NativeAccess::Fresh).unwrap();
    let stage = directory.stage();
    stage.publish().unwrap();
    assert_eq!(fs::metadata(&destination).unwrap().nlink(), 2);
    stage.finish_native_publication().unwrap();
    assert!(!stage.path().exists());
    assert_eq!(fs::metadata(&destination).unwrap().nlink(), 1);
    assert_eq!(fs::read(&destination).unwrap(), b"validated native bytes");
    assert!(matches!(
        NativeOwnership::acquire(&destination, NativeAccess::Existing),
        Err(Error::Conflict)
    ));
    drop(stage);
    assert_eq!(fs::read(&destination).unwrap(), b"validated native bytes");
    drop(owner);
    NativeOwnership::acquire(&destination, NativeAccess::Existing).unwrap();
}

#[test]
fn native_finalization_rejects_before_publication_without_removal() {
    let directory = Directory::new();
    let stage = directory.stage();
    assert_eq!(stage.finish_native_publication(), Err(Error::Storage));
    assert!(!stage.destination.exists());
    assert_eq!(fs::read(stage.path()).unwrap(), b"validated native bytes");
    // Matching aliases alone do not authorize finalization before Stage publishes.
    fs::hard_link(stage.path(), &stage.destination).unwrap();
    assert_eq!(stage.finish_native_publication(), Err(Error::Storage));
    assert!(stage.path().exists());
}

#[test]
fn native_finalization_rejects_replaced_destination_without_removal() {
    let directory = Directory::new();
    let stage = directory.stage();
    stage.publish().unwrap();
    fs::remove_file(&stage.destination).unwrap();
    fs::write(&stage.destination, b"unrelated file").unwrap();
    assert_eq!(stage.finish_native_publication(), Err(Error::Unknown));
    assert_eq!(fs::read(stage.path()).unwrap(), b"validated native bytes");
    assert_eq!(fs::read(&stage.destination).unwrap(), b"unrelated file");
}

#[test]
fn native_finalization_refuses_unknown_additional_hard_links() {
    let directory = Directory::new();
    let stage = directory.stage();
    stage.publish().unwrap();
    let unrelated = directory.path("unknown-link");
    fs::hard_link(&stage.destination, &unrelated).unwrap();
    assert_eq!(stage.finish_native_publication(), Err(Error::Unknown));
    assert!(stage.path().exists());
    assert!(stage.destination.exists());
    assert_eq!(fs::metadata(&unrelated).unwrap().nlink(), 3);
    assert_eq!(fs::read(&unrelated).unwrap(), b"validated native bytes");
}

#[test]
fn native_finalization_rejects_symlinks_and_missing_private_artifacts() {
    for replace_destination in [false, true] {
        let directory = Directory::new();
        let stage = directory.stage();
        stage.publish().unwrap();
        let (replaced, target) = if replace_destination {
            (&stage.destination, stage.path())
        } else {
            (&stage.path, stage.destination.as_path())
        };
        fs::remove_file(replaced).unwrap();
        assert_eq!(stage.finish_native_publication(), Err(Error::Unknown));
        symlink(target, replaced).unwrap();
        assert_eq!(stage.finish_native_publication(), Err(Error::Unknown));
        assert!(
            fs::symlink_metadata(replaced)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(target).unwrap(), b"validated native bytes");
    }
}

struct Process(Child);
impl Process {
    fn run(directory: &Directory, phase: &str) {
        let mut process = Self(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "publication::tests::native_publication_child",
                    "--nocapture",
                ])
                .env("ROM_PUBLICATION_CHILD_DIRECTORY", &directory.0)
                .env("ROM_PUBLICATION_CHILD_PHASE", phase)
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = process.0.try_wait().unwrap() {
                assert_eq!(status.code(), Some(86), "publication child failed");
                break;
            }
            assert!(Instant::now() < deadline, "publication child timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn native_publication_child() {
    let Some(path) = std::env::var_os("ROM_PUBLICATION_CHILD_DIRECTORY") else {
        return;
    };
    let directory = Directory(PathBuf::from(path));
    let owner = NativeOwnership::acquire(&directory.path("database"), NativeAccess::Fresh).unwrap();
    let stage = Stage::new(owner.path()).unwrap();
    fs::write(stage.path(), b"validated native bytes").unwrap();
    fs::write(
        directory.path("owned-artifact"),
        stage.path().as_os_str().as_bytes(),
    )
    .unwrap();
    stage
        .publish_with(|| {
            if std::env::var("ROM_PUBLICATION_CHILD_PHASE").unwrap() == "before" {
                std::process::exit(86);
            }
            Ok(())
        })
        .unwrap();
    // publish_with synced the destination directory; finalization has not run.
    std::process::exit(86);
}

#[test]
fn native_publication_exit_before_link_leaves_destination_absent() {
    let _process_boundary = PROCESS_BOUNDARY.lock().unwrap();
    let directory = Directory::new();
    Process::run(&directory, "before");
    let destination = directory.path("database");
    assert!(!destination.exists());
    NativeOwnership::acquire(&destination, NativeAccess::Fresh).unwrap();
}

#[test]
fn native_publication_exit_after_link_requires_exact_artifact_recovery() {
    let _process_boundary = PROCESS_BOUNDARY.lock().unwrap();
    let directory = Directory::new();
    Process::run(&directory, "after");
    let destination = directory.path("database");
    let artifact = PathBuf::from(std::ffi::OsString::from_vec(
        fs::read(directory.path("owned-artifact")).unwrap(),
    ));
    assert!(matches!(
        NativeOwnership::acquire(&destination, NativeAccess::Existing),
        Err(Error::Unsupported(_))
    ));
    let mut lock_path = destination.as_os_str().to_owned();
    lock_path.push(".rom-owner");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(PathBuf::from(lock_path))
        .unwrap();
    lock.try_lock().unwrap();
    // Recover only the known private artifact after checking its exact identity.
    assert_eq!(artifact.file_name().unwrap(), "data");
    let staging_directory = artifact.parent().unwrap();
    assert_eq!(staging_directory.parent().unwrap(), directory.0);
    assert!(
        staging_directory
            .file_name()
            .unwrap()
            .as_bytes()
            .starts_with(b".rom-maintenance-")
    );
    let source = fs::symlink_metadata(&artifact).unwrap();
    let published = fs::symlink_metadata(&destination).unwrap();
    assert!(source.is_file() && published.is_file());
    assert_eq!(
        (source.dev(), source.ino(), source.nlink()),
        (published.dev(), published.ino(), 2)
    );
    assert_eq!(published.nlink(), 2);
    fs::remove_file(&artifact).unwrap();
    File::open(staging_directory).unwrap().sync_all().unwrap();
    assert_eq!(fs::metadata(&destination).unwrap().nlink(), 1);
    drop(lock);
    NativeOwnership::acquire(&destination, NativeAccess::Existing).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"validated native bytes");
}
