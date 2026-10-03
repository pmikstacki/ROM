//! Struct positional decoding must not widen the operator envelope contract.
use super::{common::*, support};
use serde_json::{Value, json};

fn sequence(object: Value, fields: &[&str]) -> Value {
    Value::Array(fields.iter().map(|field| object[*field].clone()).collect())
}

#[tokio::test]
async fn operator_input_files_reject_top_level_positional_arrays() {
    let mut codes = Vec::new();
    for (command, input, output) in [
        (
            "list",
            json!([null, null, null, 64, null]),
            support::page(vec![]),
        ),
        (
            "retry",
            sequence(
                support::request(false),
                &["handle", "expected", "key", "retry_epoch", "operation"],
            ),
            support::result(false),
        ),
        (
            "reconcile",
            sequence(
                support::request(true),
                &["handle", "expected", "key", "retry_epoch", "operation"],
            ),
            support::result(true),
        ),
    ] {
        let (endpoint, task) =
            response(200, "application/json", output.to_string().into_bytes()).await;
        let flag = if command == "list" {
            "--query-file"
        } else {
            "--request-file"
        };
        let out = run(
            &endpoint,
            None,
            &["work", command, flag, "-"],
            &input.to_string(),
        )
        .await;
        codes.push((
            out.status.code(),
            out.stdout.is_empty(),
            !task.is_finished(),
        ));
        task.abort();
        let _ = task.await;
    }
    assert_eq!(codes, [(Some(2), true, true); 3]);
}

#[tokio::test]
async fn operator_responses_reject_top_level_positional_arrays() {
    let read = vec!["work", "show", support::handle().as_str()]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut codes = Vec::new();
    let cases = [
        (
            vec!["work".into(), "capabilities".into()],
            json!([1, false, false, false]),
            String::new(),
            4,
        ),
        (
            vec!["work".into(), "list".into()],
            json!([1, [], null]),
            String::new(),
            4,
        ),
        (
            read,
            sequence(
                support::view(),
                &[
                    "protocol_version",
                    "handle",
                    "version",
                    "category",
                    "definition",
                    "state",
                    "attempts",
                    "due",
                    "delivery",
                    "source",
                    "target",
                ],
            ),
            String::new(),
            4,
        ),
        (
            vec![
                "work".into(),
                "retry".into(),
                "--request-file".into(),
                "-".into(),
            ],
            sequence(
                support::result(false),
                &[
                    "protocol_version",
                    "handle",
                    "version",
                    "key",
                    "operation",
                    "outcome",
                    "replayed",
                ],
            ),
            support::request(false).to_string(),
            5,
        ),
        (
            vec![
                "work".into(),
                "reconcile".into(),
                "--request-file".into(),
                "-".into(),
            ],
            sequence(
                support::result(true),
                &[
                    "protocol_version",
                    "handle",
                    "version",
                    "key",
                    "operation",
                    "outcome",
                    "replayed",
                ],
            ),
            support::request(true).to_string(),
            5,
        ),
    ];
    for (args, output, input, expected) in cases {
        let (endpoint, task) =
            response(200, "application/json", output.to_string().into_bytes()).await;
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let out = run(&endpoint, None, &args, &input).await;
        codes.push((out.status.code(), Some(expected), out.stdout.is_empty()));
        task.await.unwrap();
    }
    assert!(
        codes
            .iter()
            .all(|(code, expected, empty)| code == expected && *empty),
        "{codes:?}"
    );
}
