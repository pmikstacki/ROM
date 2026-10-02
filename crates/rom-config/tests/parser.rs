use rom_config::{Format, parse};
#[test]
fn both_formats_preserve_types_and_literal_fields_without_path_merging() {
    for (format, body) in [
        (
            Format::Json,
            r#"{"enabled":false,"count":0,"empty":"","tags":["one"],"literal.key":"value"}"#,
        ),
        (
            Format::Toml,
            "enabled=false\ncount=0\nempty=''\ntags=['one']\n'literal.key'='value'\n",
        ),
    ] {
        let parsed = parse(format, body, "deployment").unwrap();
        assert_eq!(
            parsed.values(),
            &rom::json!({"enabled":false,"count":0,"empty":"","tags":["one"],"literal.key":"value"})
        );
        assert_eq!(
            parsed.origins().get("literal.key").map(String::as_str),
            Some("deployment")
        );
    }
    assert_eq!(
        parse(Format::Json, "{\"optional\":null}", "deployment")
            .unwrap()
            .values(),
        &rom::json!({"optional":null})
    );
}
#[test]
fn ambiguous_duplicate_values_and_nonobjects_are_rejected() {
    for body in [
        "{\"enabled\":\"bad\",\"enabled\":true}",
        "{\"nested\":{\"x\":1,\"x\":2}}",
        "[]",
        "null",
        "true",
    ] {
        assert!(parse(Format::Json, body, "deployment").is_err());
    }
    assert!(parse(Format::Toml, "enabled=false\nenabled=true", "deployment").is_err());
    assert!(parse(Format::Toml, "number=nan", "deployment").is_err());
    assert!(parse(Format::Toml, "number=inf", "deployment").is_err());
}
#[test]
fn parser_errors_and_debug_do_not_disclose_values_or_paths() {
    let secret = "sensitive-marker-never-log";
    let error = parse(Format::Json, &format!("{{\"key\":{secret}}}"), "deployment").unwrap_err();
    assert!(!format!("{error:?} {error}").contains(secret));
    let parsed = parse(
        Format::Json,
        &format!("{{\"key\":\"{secret}\"}}"),
        "deployment",
    )
    .unwrap();
    assert!(!format!("{parsed:?}").contains(secret));
    assert!(parse(Format::Json, "{}", "/secret/path/config.json").is_err());
    assert!(parse(Format::Json, &" ".repeat(65_537), "deployment").is_err());
}
