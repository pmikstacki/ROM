//! Operator controls reuse the independent signal task even when output blocks.
use super::{Directory, request, support};
use std::{
    io::Write,
    os::{fd::OwnedFd, unix::net::UnixStream},
    process::{Command, Stdio},
    time::Duration,
};

fn blocked_output() -> (Stdio, UnixStream) {
    let (mut writer, reader) = UnixStream::pair().unwrap();
    writer.set_nonblocking(true).unwrap();
    loop {
        match writer.write(&[b'x'; 8192]) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(error) => panic!("{error}"),
        }
    }
    writer.set_nonblocking(false).unwrap();
    let fd: OwnedFd = writer.into();
    (Stdio::from(fd), reader)
}
#[tokio::test]
async fn control_interrupt_does_not_wait_on_blocked_stdout_or_stderr() {
    for acknowledgement in [false, true] {
        let dir = Directory::new();
        let file = dir.file("request", support::request(false).to_string());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (blocked, _reader) = blocked_output();
        let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_rom"));
        child.args([
            "--endpoint",
            &endpoint,
            "work",
            "retry",
            "--request-file",
            &file,
        ]);
        if acknowledgement {
            child.stdout(blocked).stderr(Stdio::null());
        } else {
            child.stdout(Stdio::null()).stderr(blocked);
        }
        let mut child = child.kill_on_drop(true).spawn().unwrap();
        let (mut socket, _) = tokio::time::timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(request(&mut socket).await, support::request(false));
        if acknowledgement {
            support::reply(&mut socket, 200, &support::result(false)).await;
        } else {
            support::reply(
                &mut socket,
                503,
                &serde_json::json!({"error":"outcome_unknown"}),
            )
            .await;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            Command::new("kill")
                .args(["-INT", &child.id().unwrap().to_string()])
                .status()
                .unwrap()
                .success()
        );
        let status = tokio::time::timeout(Duration::from_secs(2), child.wait())
            .await
            .expect("Ctrl-C must not wait on output")
            .unwrap();
        assert_eq!(status.code(), Some(if acknowledgement { 0 } else { 130 }));
    }
}
