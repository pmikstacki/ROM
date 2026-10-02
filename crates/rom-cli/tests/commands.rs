use std::io::Write;
use std::process::{Command, Stdio};
fn run(args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rom"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if !input.is_empty() {
        let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
    }
    child.wait_with_output().unwrap()
}
#[test]
fn mutation_requires_explicit_key_and_revision() {
    for args in [
        vec!["create", "tasks", "id", "--input-file", "-"],
        vec!["delete", "tasks", "id", "--idempotency", "key"],
    ] {
        let mut all = vec!["--endpoint", "http://127.0.0.1:1"];
        all.extend(args);
        let out = run(&all, "{}");
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn invalid_urls_and_payloads_fail_before_network_without_echoing_input() {
    for endpoint in [
        "http://example.com",
        "http://secret@example.com",
        "http://127.0.0.1/?password=secret",
        "http://127.0.0.1/#secret",
    ] {
        let out = run(&["--endpoint", endpoint, "read", "tasks", "id"], "");
        assert_eq!(out.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&out.stderr).contains("secret"));
    }
    for payload in [
        r#"{"a":{"secret":1,"secret":2}}"#.to_string(),
        "x".repeat(65_537),
    ] {
        let out = run(
            &[
                "--endpoint",
                "http://127.0.0.1:1",
                "create",
                "tasks",
                "id",
                "--idempotency",
                "key",
                "--input-file",
                "-",
            ],
            &payload,
        );
        assert_eq!(out.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&out.stderr).contains("secret"));
    }
}
#[test]
fn help_lists_generic_operations_and_recovery_contract() {
    let out = run(&["--help"], "");
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    for command in [
        "discover",
        "read",
        "query",
        "create",
        "replace",
        "patch",
        "delete",
        "action",
        "invoke",
        "live",
        "journal",
        "journal-head",
        "subscribe",
    ] {
        assert!(help.contains(command), "{command}");
    }
}
