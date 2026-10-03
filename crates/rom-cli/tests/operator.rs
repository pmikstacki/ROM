//! Client conformance plus native-backed HTTP operator journeys.
mod common;
#[path = "operator/native.rs"]
mod native;
#[path = "operator/native_support.rs"]
mod native_support;
#[path = "operator/object_shape.rs"]
mod object_shape;
#[path = "operator/reconciliation.rs"]
mod reconciliation;
#[path = "operator/support.rs"]
mod support;
use common::*;
use serde_json::json;
use support::{handle, page, reply, result, view};

#[tokio::test]
async fn work_commands_map_exactly_to_public_operator_requests() {
    let dir = Directory::new();
    let auth = dir.file("auth", "Bearer operator-session");
    let query = json!({"state":"Pending","category":"Notification","definition":"notice","limit":128,"cursor":"opaque-cursor"});
    let cases = vec![
        (
            vec!["work".to_owned(), "capabilities".into()],
            "/work/capabilities",
            json!({}),
            json!({"protocol_version":1,"inspect":false,"retry":false,"reconcile":false}),
        ),
        (
            vec!["work".into(), "list".into()],
            "/work/list",
            json!({"state":null,"category":null,"definition":null,"limit":64,"cursor":null}),
            page(vec![]),
        ),
        (
            vec![
                "work".into(),
                "list".into(),
                "--query-file".into(),
                "-".into(),
            ],
            "/work/list",
            query.clone(),
            page(vec![view()]),
        ),
        (
            vec!["work".into(), "show".into(), handle().as_str().into()],
            "/work/read",
            json!({"handle":handle()}),
            view(),
        ),
        (
            vec![
                "work".into(),
                "retry".into(),
                "--request-file".into(),
                "-".into(),
            ],
            "/work/control",
            support::request(false),
            result(false),
        ),
        (
            vec![
                "work".into(),
                "reconcile".into(),
                "--request-file".into(),
                "-".into(),
            ],
            "/work/control",
            support::request(true),
            result(true),
        ),
    ];
    for (args, route, expected, output) in cases {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let wanted = expected.clone();
        let response = output.clone();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let header = support::header(&socket).await;
            assert!(
                header.starts_with(&format!("POST {route} HTTP/1.1\r\n")),
                "{header}"
            );
            assert!(
                header
                    .to_ascii_lowercase()
                    .contains("authorization: bearer operator-session")
            );
            assert_eq!(request(&mut socket).await, wanted);
            reply(&mut socket, 200, &response).await;
        });
        let argv: Vec<_> = args.iter().map(String::as_str).collect();
        let input = if argv.contains(&"-") {
            expected.to_string()
        } else {
            String::new()
        };
        let out = run(&endpoint, Some(&auth), &argv, &input).await;
        assert_eq!(json(&out), output);
        task.await.unwrap();
    }
}
#[tokio::test]
async fn malformed_operator_files_and_wrong_operations_fail_before_submission() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let mut unknown = support::request(false);
    unknown["payload"] = json!("sentinel-secret");
    let mut oversize = support::request(false);
    oversize["key"] = json!("k".repeat(1025));
    let mut version = support::request(false);
    version["expected"]["generation"] = json!("g".repeat(129));
    let inputs = [
        unknown.to_string(),
        oversize.to_string(),
        version.to_string(),
        support::request(true).to_string(),
        "{\"key\":\"a\",\"key\":\"b\"}".into(),
    ];
    for input in inputs {
        let out = run(
            &endpoint,
            None,
            &["work", "retry", "--request-file", "-"],
            &input,
        )
        .await;
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("sentinel-secret"));
    }
    let out = run(
        &endpoint,
        None,
        &["work", "reconcile", "--request-file", "-"],
        &support::request(false).to_string(),
    )
    .await;
    assert_eq!(out.status.code(), Some(2));
    for input in [
        json!({"limit":0}),
        json!({"limit":1,"unknown":true}),
        json!({"limit":usize::MAX}),
    ] {
        assert_eq!(
            run(
                &endpoint,
                None,
                &["work", "list", "--query-file", "-"],
                &input.to_string()
            )
            .await
            .status
            .code(),
            Some(2)
        );
    }
    assert_eq!(
        run(&endpoint, None, &["work", "show", "sentinel-secret"], "")
            .await
            .status
            .code(),
        Some(2)
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), listener.accept())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn control_responses_match_request_identity_operation_and_version_transition() {
    for reconcile in [false, true] {
        for field in [
            "handle",
            "key",
            "operation",
            "protocol",
            "generation",
            "revision",
            "outcome",
            "unknown",
        ] {
            let mut invalid = result(reconcile);
            match field {
                "handle" => {
                    invalid[field] = json!(rom::operator::WorkHandle::from_work_id("wrong-target"))
                }
                "key" => invalid[field] = json!("wrong-key"),
                "operation" => invalid[field] = support::request(!reconcile)[field].clone(),
                "protocol" => invalid["protocol_version"] = json!(2),
                "generation" => invalid["version"]["generation"] = json!("restored"),
                "revision" => invalid["version"]["revision"] = json!(6),
                "outcome" => {
                    invalid[field] = json!("Unresolved");
                    invalid["replayed"] = json!(true);
                }
                _ => invalid["provider_body"] = json!("sentinel-provider-secret"),
            }
            let (endpoint, task) =
                response(200, "application/json", invalid.to_string().into_bytes()).await;
            let out = run(
                &endpoint,
                None,
                &[
                    "work",
                    if reconcile { "reconcile" } else { "retry" },
                    "--request-file",
                    "-",
                ],
                &support::request(reconcile).to_string(),
            )
            .await;
            assert_eq!(out.status.code(), Some(5), "{field}");
            assert!(out.stdout.is_empty());
            assert!(!String::from_utf8_lossy(&out.stderr).contains("sentinel-provider-secret"));
            task.await.unwrap();
        }
    }
}
#[tokio::test]
async fn list_and_show_responses_respect_scope_filters_uniqueness_and_limits() {
    let query =
        json!({"state":"Pending","category":"Notification","definition":"notice","limit":1});
    for field in [
        "state",
        "category",
        "definition",
        "duplicates",
        "count",
        "protocol",
        "oversize",
    ] {
        let mut invalid = page(vec![view()]);
        match field {
            "state" => invalid["records"][0]["state"] = json!("Done"),
            "category" => invalid["records"][0]["category"] = json!("Reaction"),
            "definition" => invalid["records"][0]["definition"]["name"] = json!("other"),
            "duplicates" => invalid["records"].as_array_mut().unwrap().push(view()),
            "count" => {
                let mut second = view();
                second["handle"] = json!(rom::operator::WorkHandle::from_work_id("second"));
                invalid["records"].as_array_mut().unwrap().push(second);
            }
            "protocol" => invalid["records"][0]["protocol_version"] = json!(2),
            _ => {
                invalid["records"][0]["source"] =
                    json!({"kind":"notice","id":"x".repeat(2*1024*1024)})
            }
        }
        let (endpoint, task) =
            response(200, "application/json", invalid.to_string().into_bytes()).await;
        let out = run(&endpoint, None, &["work", "list", "--query-file", "-"], &{
            let mut query = query.clone();
            if field == "duplicates" {
                query["limit"] = json!(2);
            }
            query.to_string()
        })
        .await;
        assert_eq!(out.status.code(), Some(4), "{field}");
        assert!(out.stdout.is_empty());
        task.await.unwrap();
    }
    let mut wrong = view();
    wrong["handle"] = json!(rom::operator::WorkHandle::from_work_id("other"));
    let (endpoint, task) = response(200, "application/json", wrong.to_string().into_bytes()).await;
    let out = run(&endpoint, None, &["work", "show", handle().as_str()], "").await;
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    task.await.unwrap();
}

#[tokio::test]
async fn reconcile_outcomes_and_exact_replay_have_their_declared_versions() {
    for (outcome, revision, replayed) in [
        (json!("Completed"), 5, false),
        (json!("Scheduled"), 5, false),
        (json!({"Stopped":"Attempts"}), 5, false),
        (json!("Completed"), 5, true),
        (json!("Unresolved"), 4, false),
    ] {
        let mut body = result(true);
        body["outcome"] = outcome;
        body["version"]["revision"] = json!(revision);
        body["replayed"] = json!(replayed);
        let (endpoint, task) =
            response(200, "application/json", body.to_string().into_bytes()).await;
        let out = run(
            &endpoint,
            None,
            &["work", "reconcile", "--request-file", "-"],
            &support::request(true).to_string(),
        )
        .await;
        assert_eq!(json(&out), body);
        task.await.unwrap();
    }
    for (reconcile, outcome, revision, replayed) in [
        (false, json!("Completed"), 5, false),
        (false, json!({"Stopped":"Attempts"}), 5, false),
        (true, json!("Unresolved"), 5, false),
        (true, json!("Unresolved"), 4, true),
        (true, json!("Completed"), 4, false),
    ] {
        let mut body = result(reconcile);
        body["outcome"] = outcome;
        body["version"]["revision"] = json!(revision);
        body["replayed"] = json!(replayed);
        let (endpoint, task) =
            response(200, "application/json", body.to_string().into_bytes()).await;
        let out = run(
            &endpoint,
            None,
            &[
                "work",
                if reconcile { "reconcile" } else { "retry" },
                "--request-file",
                "-",
            ],
            &support::request(reconcile).to_string(),
        )
        .await;
        assert_eq!(out.status.code(), Some(5));
        assert!(out.stdout.is_empty());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn lost_control_reply_replays_the_exact_file_only_after_an_explicit_new_command() {
    let dir = Directory::new();
    let file = dir.file("request", support::request(false).to_string());
    let auth = dir.file("auth", "Bearer original-principal");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut first, _) = listener.accept().await.unwrap();
        let principal = support::header(&first).await;
        let submitted = request(&mut first).await;
        assert_eq!(submitted, support::request(false));
        drop(first);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
        ready_tx.send(()).unwrap();
        let (mut second, _) = listener.accept().await.unwrap();
        assert_eq!(support::header(&second).await, principal);
        assert_eq!(request(&mut second).await, submitted);
        let mut replay = result(false);
        replay["replayed"] = json!(true);
        reply(&mut second, 200, &replay).await;
    });
    let args = ["work", "retry", "--request-file", file.as_str()];
    let unknown = run(&endpoint, Some(&auth), &args, "").await;
    assert_eq!(unknown.status.code(), Some(5));
    assert!(unknown.stdout.is_empty());
    ready_rx.await.unwrap();
    let accepted = run(&endpoint, Some(&auth), &args, "").await;
    assert_eq!(json(&accepted)["replayed"], true);
    task.await.unwrap();
}
#[tokio::test]
async fn capability_denial_and_untrusted_provider_errors_remain_sanitized() {
    for (args, status, body, code) in [
        (
            vec!["work", "capabilities"],
            403,
            json!({"error":"denied","details":"sentinel-secret"}),
            3,
        ),
        (vec!["work", "list"], 400, json!({"error":"invalid"}), 3),
        (vec!["work", "list"], 501, json!({"error":"unsupported"}), 4),
        (
            vec!["work", "retry", "--request-file", "-"],
            403,
            json!({"error":"denied","details":"sentinel-secret"}),
            5,
        ),
        (
            vec!["work", "reconcile", "--request-file", "-"],
            500,
            json!({"error":"sentinel-secret"}),
            5,
        ),
    ] {
        let (endpoint, task) =
            response(status, "application/json", body.to_string().into_bytes()).await;
        let out = run(
            &endpoint,
            None,
            &args,
            &support::request(args.contains(&"reconcile")).to_string(),
        )
        .await;
        assert_eq!(out.status.code(), Some(code));
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("sentinel-secret"));
        task.await.unwrap();
    }
    let (endpoint, task) = response(
        200,
        "application/json",
        json!({"protocol_version":2,"inspect":true,"retry":true,"reconcile":true})
            .to_string()
            .into_bytes(),
    )
    .await;
    assert_eq!(
        run(&endpoint, None, &["work", "capabilities"], "")
            .await
            .status
            .code(),
        Some(4)
    );
    task.await.unwrap();
}
#[cfg(unix)]
#[path = "operator/signals.rs"]
mod signals;

#[tokio::test]
async fn configured_host_page_above_the_default_still_fits_the_cli_bounds() {
    let records = (0..65)
        .map(|n| {
            let mut value = view();
            value["handle"] = json!(rom::operator::WorkHandle::from_work_id(&format!(
                "work-{n}"
            )));
            value
        })
        .collect();
    let expected = page(records);
    let (endpoint, task) =
        response(200, "application/json", expected.to_string().into_bytes()).await;
    let out = run(
        &endpoint,
        None,
        &["work", "list", "--query-file", "-"],
        r#"{"limit":128}"#,
    )
    .await;
    assert_eq!(json(&out), expected);
    task.await.unwrap();
}

#[tokio::test]
async fn control_revision_transition_cannot_wrap_at_the_numeric_limit() {
    let mut input = support::request(false);
    input["expected"]["revision"] = json!(u64::MAX);
    let mut output = result(false);
    output["version"]["revision"] = json!(0);
    let (endpoint, task) = response(200, "application/json", output.to_string().into_bytes()).await;
    let out = run(
        &endpoint,
        None,
        &["work", "retry", "--request-file", "-"],
        &input.to_string(),
    )
    .await;
    assert_eq!(out.status.code(), Some(5));
    assert!(out.stdout.is_empty());
    task.await.unwrap();
}
