use rom::{Error, Value, json, parse_json};

#[test]
fn public_decoder_accepts_complete_json_roots_and_preserves_values() {
    for (bytes, expected) in [
        (
            br#"{"nested":[true,null,0,-1,1.5,"text"]}"#.as_slice(),
            json!({"nested":[true,null,0,-1,1.5,"text"]}),
        ),
        (b"[1,2]".as_slice(), json!([1, 2])),
        (b"null".as_slice(), Value::Null),
        (b"false".as_slice(), json!(false)),
        (b"42".as_slice(), json!(42)),
        (br#""scalar""#.as_slice(), json!("scalar")),
    ] {
        assert_eq!(parse_json(bytes).unwrap(), expected);
    }
}

#[test]
fn duplicate_keys_fail_at_any_depth_with_one_safe_error() {
    for input in [
        br#"{"a":1,"a":2}"#.as_slice(),
        br#"{"nested":{"a":1,"a":2}}"#.as_slice(),
        br#"[{"a":1,"a":2}]"#.as_slice(),
        br#"{"a":1,"\u0061":2}"#.as_slice(),
        br#"{"private-marker":1,"private-marker":2}"#.as_slice(),
    ] {
        assert_eq!(parse_json(input), Err(Error::invalid("request", "json")));
    }
}

#[test]
fn invalid_trailing_and_excessively_nested_input_share_safe_errors() {
    for input in [
        b"".as_slice(),
        b"null null",
        b"{}1",
        b"1e400",
        br#"{"private-marker":"secret""#,
    ] {
        assert_eq!(parse_json(input), Err(Error::invalid("request", "json")));
    }
    let nested = format!("{}null{}", "[".repeat(200), "]".repeat(200));
    assert_eq!(
        parse_json(nested.as_bytes()),
        Err(Error::invalid("request", "json"))
    );
}
