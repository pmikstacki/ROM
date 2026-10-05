use rom::{CodecWrapper, Field, Presence, Resource, Value, json};
use rom_conformance::{CodecCase, field::codec};
use rom_fields::*;

fn vectors<F: Field + PartialEq + std::fmt::Debug>(pairs: &[(&str, &str)], invalid: &[&str]) {
    let cases: Vec<_> = pairs
        .iter()
        .map(|(input, canonical)| CodecCase {
            input: json!(input),
            canonical: json!(canonical),
            expected: F::decode(json!(canonical)).unwrap(),
        })
        .collect();
    let invalid: Vec<_> = invalid
        .iter()
        .map(|v| json!(v))
        .chain([Value::Null, json!(false), json!(123), json!({}), json!([])])
        .collect();
    codec::<F>(&cases, &invalid).unwrap();
}
#[test]
fn calendar_and_time_boundaries() {
    vectors::<Date>(
        &[
            ("2000-02-29", "2000-02-29"),
            ("0001-01-01", "0001-01-01"),
            ("9999-12-31", "9999-12-31"),
        ],
        &[
            "1900-02-29",
            "2025-02-29",
            "0000-01-01",
            "2026-2-01",
            "2026-04-31",
            "２０２６-01-01",
            "2026-01-01Z",
        ],
    );
    vectors::<Time>(
        &[
            ("00:00:00", "00:00:00"),
            ("23:59:59.123456789", "23:59:59.123456789"),
            ("12:30:10.12000", "12:30:10.12"),
            ("00:00:00.000", "00:00:00"),
        ],
        &[
            "24:00:00",
            "12:60:00",
            "23:59:60",
            "12:30",
            "12:30:00Z",
            "12:30:00.",
            "12:30:00.1234567890",
        ],
    );
}
#[test]
fn timestamps_are_known_instants_with_chronological_string_order() {
    vectors::<DateTime>(
        &[
            (
                "2024-03-31T01:30:00+01:00",
                "2024-03-31T00:30:00.000000000Z",
            ),
            (
                "2024-03-31T02:30:00+02:00",
                "2024-03-31T00:30:00.000000000Z",
            ),
            (
                "2026-10-05T12:30:00.1200-05:30",
                "2026-10-05T18:00:00.120000000Z",
            ),
        ],
        &[
            "2026-10-05T12:00:00",
            "2026-10-05T12:00:00-00:00",
            "2026-10-05 12:00:00Z",
            "2026-10-05T12:00:60Z",
            "0001-01-01T00:00:00+01:00",
            "9999-12-31T23:59:59-01:00",
            "2026-10-05T12:00:00+24:00",
            "2026-10-05T12:00:00+00:99",
        ],
    );
    let before = DateTime::new("2026-10-05T12:00:00.01Z").unwrap();
    let after = DateTime::new("2026-10-05T12:00:00.1Z").unwrap();
    assert!(before.as_str() < after.as_str());
}
#[test]
fn colors_and_mailboxes() {
    vectors::<Color>(
        &[("#ABCdef", "#abcdef"), ("#112233FF", "#112233ff")],
        &["red", "#abc", "#gg0000", "#1122334", "#112233ff "],
    );
    vectors::<Email>(
        &[
            ("Alice+tag@EXAMPLE.COM", "Alice+tag@example.com"),
            ("a.b@example.test", "a.b@example.test"),
        ],
        &[
            "",
            "a..b@example.test",
            ".a@example.test",
            "a@-example.test",
            "a@example..test",
            "a@b@c",
            "Display <a@b.test>",
            "a\r\n@b.test",
            "a@é.test",
        ],
    );
}
#[test]
fn urls_are_parsed_without_active_schemes_or_credentials() {
    vectors::<Url>(
        &[
            ("HTTPS://EXAMPLE.COM:443/a", "https://example.com/a"),
            ("https://example.test", "https://example.test/"),
            ("http://[::1]/", "http://[::1]/"),
        ],
        &[
            "javascript:alert(1)",
            "file:///etc/passwd",
            "/relative",
            "https://a:b@example.test/",
            "https://example.test/\n",
            "https://example.test/a b",
            "https:\\example.test",
            "https:///",
            "https://example.test:99999",
        ],
    );
}
#[test]
fn exact_text_and_json_do_not_rewrite_precision_or_line_endings() {
    vectors::<Multiline>(&[("", ""), ("one\r\ntwo\n", "one\r\ntwo\n")], &[]);
    vectors::<JsonDocument>(
        &[
            (
                " {\"n\":123456789012345678901234567890.12345678901234567890} ",
                " {\"n\":123456789012345678901234567890.12345678901234567890} ",
            ),
            ("null", "null"),
            ("[true, false]", "[true, false]"),
        ],
        &["", "{", "NaN", "[1,]", "true false"],
    );
    assert!(Multiline::new("x".repeat(1_048_577)).is_err());
    assert!(JsonDocument::new(format!("\"{}\"", "x".repeat(1_048_576))).is_err());
}
#[test]
fn decimal_and_units_remain_exact_strings() {
    vectors::<Decimal>(
        &[
            ("000123.45000", "123.45"),
            ("-000.000", "0"),
            ("-00.00100", "-0.001"),
            (
                "123456789012345678901234567890.1234567890123456789",
                "123456789012345678901234567890.1234567890123456789",
            ),
        ],
        &["1e3", "+1", " 1", "1.", ".1", "NaN", "", "1.2.3"],
    );
    assert!(Decimal::new("1".repeat(1025)).is_err());
    let value = UnitValue::new(Decimal::new("001.20").unwrap(), "kg").unwrap();
    codec(
        &[CodecCase {
            input: json!({"value":"001.20","unit":"kg"}),
            canonical: json!({"value":"1.2","unit":"kg"}),
            expected: value,
        }],
        &[
            json!({"value":1.2,"unit":"kg"}),
            json!({"value":"1","unit":""}),
            json!({"value":"1","unit":"kg","extra":"x"}),
            json!({"value":"1"}),
            json!({"value":"1","unit":"kg m"}),
        ],
    )
    .unwrap();
}
#[derive(Clone, Resource)]
#[resource(name = "semantic")]
struct Semantic {
    date: Date,
    decimal: Decimal,
    maybe: Presence<Option<Vec<Date>>>,
}
#[test]
fn public_resource_normalization_enforces_semantics_and_wrapper_contracts() {
    assert!(Semantic::normalize_field("date", json!("2025-02-29")).is_err());
    assert!(Semantic::normalize_field("decimal", json!(1)).is_err());
    assert_eq!(
        Semantic::normalize_field("decimal", json!("001.200")).unwrap(),
        json!("1.2")
    );
    assert_eq!(
        Semantic::normalize_field("maybe", Value::Null).unwrap(),
        Value::Null
    );
    assert!(Semantic::normalize_field("maybe", json!(["2025-02-29"])).is_err());
    assert!(
        !<Presence<Option<Vec<Date>>>>::decode_missing()
            .unwrap()
            .is_present()
    );
    assert_eq!(
        <Presence<Option<Vec<Date>>>>::codec_wrappers(),
        vec![
            CodecWrapper::Optional,
            CodecWrapper::Nullable,
            CodecWrapper::List
        ]
    );
    for binding in Semantic::field_codecs() {
        assert_eq!(binding.codec.version, 1);
    }
    assert!(Date::decode_missing().is_err());
    assert!(Date::decode(Value::Null).is_err());
}

#[test]
fn native_profile_and_broken_canonical_fixture_fail_explicitly() {
    rom_conformance::profile::require(1).unwrap();
    assert!(rom_conformance::profile::require(2).is_err());
    assert!(
        codec::<Decimal>(
            &[CodecCase {
                input: json!("001.20"),
                canonical: json!("1.200"),
                expected: Decimal::new("1.2").unwrap(),
            }],
            &[],
        )
        .is_err()
    );
}
