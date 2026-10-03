use std::process::Command;

#[test]
fn operator_command_recovers_work_then_runs_explicit_compensation_on_both_stores() {
    for backend in ["sqlite", "redb"] {
        let output = Command::new(env!("CARGO_BIN_EXE_rom-demo"))
            .args(["operator", backend])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("Operator recovery passed"));
    }
}
