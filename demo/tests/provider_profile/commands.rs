use super::{configuration::document, support::*};
use std::{fs::File, process::Command};
fn command(scratch: &Scratch, args: &[&str], suffix: &str) -> (bool, String, String) {
    let out = scratch.0.join(format!("out{suffix}"));
    let err = scratch.0.join(format!("err{suffix}"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_rom-demo"));
    command
        .args(args)
        .stdout(File::create(&out).unwrap())
        .stderr(File::create(&err).unwrap());
    let status = Process::spawn(command).wait();
    (
        status.success(),
        std::fs::read_to_string(out).unwrap(),
        std::fs::read_to_string(err).unwrap(),
    )
}
#[test]
fn offline_commands_on_both_stores_reject_unconfigured_targets_and_emit_only_safe_results() {
    for backend in ["sqlite", "redb"] {
        let scratch = Scratch::new();
        let config = scratch.file(
            "config",
            serde_json::to_string(&document("https://fixture.invalid/introspect"))
                .unwrap()
                .as_bytes(),
            0o600,
        );
        let db = scratch.0.join("db");
        let db = db.to_str().unwrap();
        let config = config.to_str().unwrap();
        for suffix in ["first", "replay"] {
            let (ok, out, err) = command(
                &scratch,
                &["provider-provision", backend, db, config],
                suffix,
            );
            assert!(ok, "{err}");
            assert_eq!(out.trim(), "{\"provisioned\":true}");
        }
        let invocation=scratch.file("invocation",br#"{"kind":"identity-providers","id":"provider","expected":1,"idempotency":"disable","operation":{"type":"patch","input":{"enabled":{"op":"set","value":false}}}}"#,0o600);
        let (ok, out, err) = command(
            &scratch,
            &[
                "provider-maintain",
                backend,
                db,
                config,
                invocation.to_str().unwrap(),
            ],
            "maintain",
        );
        assert!(ok, "{err}");
        assert_eq!(out.trim(), "{\"revision\":2}");
        let forbidden=scratch.file("forbidden",br#"{"kind":"users","id":"secret-path-marker","expected":1,"idempotency":"bad","operation":{"type":"delete"}}"#,0o600);
        let (ok, out, err) = command(
            &scratch,
            &[
                "provider-maintain",
                backend,
                db,
                config,
                forbidden.to_str().unwrap(),
            ],
            "bad",
        );
        assert!(!ok);
        assert!(out.is_empty());
        assert!(!err.contains("secret-path-marker"));
        assert!(err.contains("Denied"));
    }
}
#[tokio::test]
async fn offline_command_refuses_a_database_owned_by_a_live_runtime() {
    use rom_demo::provider_profile::{LocalMode, build, provision};
    use std::sync::Arc;
    for redb in [false, true] {
        let scratch = Scratch::new();
        let path = scratch.0.join("db");
        let settings = settings("https://fixture.invalid/introspect");
        let runtime = build(
            storage(redb, &path),
            Arc::new(Time),
            &settings,
            LocalMode::Provisioning,
        )
        .unwrap();
        provision(&runtime, &settings).await.unwrap();
        let config = scratch.file(
            "config",
            serde_json::to_string(&document("https://fixture.invalid/introspect"))
                .unwrap()
                .as_bytes(),
            0o600,
        );
        let (ok, out, err) = command(
            &scratch,
            &[
                "provider-provision",
                if redb { "redb" } else { "sqlite" },
                path.to_str().unwrap(),
                config.to_str().unwrap(),
            ],
            "owned",
        );
        assert!(!ok);
        assert!(out.is_empty());
        assert!(err.contains("Conflict"));
        assert!(!err.contains(path.to_str().unwrap()));
        runtime.shutdown().await.unwrap();
    }
}
#[test]
fn readiness_allows_immediate_sigint_and_serving_never_provisions() {
    // Reuse the finite Node child owner; signal synchronously on readiness data.
    let script = r#"
import { pathToFileURL } from 'node:url';
const { bounded, startProcess, stop } = await import(pathToFileURL(process.argv[1]));
for (let i = 0; i < 16; i++) {
  const owned = startProcess(process.argv[2], ['provider-serve', ...process.argv.slice(3), '0']);
  try {
    let signalled = false;
    const ready = new Promise(resolve => owned.child.stdout.on('data', () => {
      if (!signalled && owned.stdout().includes('\n')) {
        JSON.parse(owned.stdout().split('\n')[0]);
        signalled = true;
        owned.child.kill('SIGINT');
        resolve();
      }
    }));
    await bounded(ready);
    const [code, signal] = await bounded(owned.done);
    if (code !== 0 || signal !== null || owned.exceeded()) throw new Error('SIGINT did not drain');
  } finally { await stop(owned); }
}
"#;
    for backend in ["sqlite", "redb"] {
        let scratch = Scratch::new();
        let config = scratch.file(
            "config",
            serde_json::to_string(&document("https://fixture.invalid/introspect"))
                .unwrap()
                .as_bytes(),
            0o600,
        );
        let path = scratch.0.join("db");
        let out = scratch.0.join("signal-out");
        let err = scratch.0.join("signal-err");
        let mut child = Command::new("node");
        child.args(["--input-type=module", "-e", script]);
        child.arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("provider-fixture/processes.mjs"),
        );
        child.args([env!("CARGO_BIN_EXE_rom-demo"), backend]);
        child.arg(&path).arg(config);
        child
            .stdout(File::create(out).unwrap())
            .stderr(File::create(&err).unwrap());
        let status = Process::spawn(child).wait();
        assert!(
            status.success(),
            "{}",
            std::fs::read_to_string(err).unwrap()
        );
        assert_eq!(Db::open(backend == "redb", &path).counts(), [0, 0, 0, 0]);
    }
}
