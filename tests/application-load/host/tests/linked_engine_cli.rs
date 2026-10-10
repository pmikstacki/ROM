//! Exercise the final Host CLI before configuration or server startup.
use std::process::{Command, Output};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rom-application-load-authoring"))
        .args(arguments)
        .env_clear()
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .output()
        .expect("run exact Cargo-built Host")
}

#[test]
#[cfg(feature = "storage-stage-timings")]
fn final_host_reports_its_linked_engine_without_configuration() {
    let output = invoke(&["--sqlite-engine-diagnostic"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert!(output.stdout.len() <= 128 * 1024);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "rom-load-host-linked-sqlite-v1");
    assert_eq!(value["mode"], "offline-linked-engine");
    assert_eq!(value["diagnostic_only"], true);
    assert_eq!(value["acceptance"], false);
    assert_eq!(value["persisted_database"], false);
    assert_eq!(value["remote_endpoint"], false);
    assert!(value["expected_profile"].is_null());
    assert!(value["profile_matches"].is_null());
    assert!(!value["engine"]["version"].as_str().unwrap().is_empty());
    assert!(!value["engine"]["source_id"].as_str().unwrap().is_empty());
    assert!(
        !value["engine"]["compile_options"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn final_host_rejects_malformed_diagnostic_arguments_without_stdout() {
    for arguments in [
        vec!["--sqlite-engine-diagnostic", "--wrong"],
        vec![
            "--sqlite-engine-diagnostic",
            "--expect-native-3534",
            "extra",
        ],
    ] {
        let output = invoke(&arguments);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }
}
