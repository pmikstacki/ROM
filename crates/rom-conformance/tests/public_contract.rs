use rom::{Error, Field, Shape, Value, json};
use rom_conformance::{CodecCase, FailureCategory, PROFILE_VERSION, field, profile};

#[derive(Clone, PartialEq)]
struct Canonical(String);
impl Field for Canonical {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> rom::Result<Self> {
        value
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| Self(s.trim().to_owned()))
            .ok_or(Error::Denied)
    }
}
#[test]
fn public_codec_uses_explicit_canonical_and_typed_values() {
    profile::require(PROFILE_VERSION).unwrap();
    field::codec(
        &[CodecCase {
            input: json!(" value "),
            canonical: json!("value"),
            expected: Canonical("value".into()),
        }],
        &[json!(""), json!(false)],
    )
    .unwrap();
}
#[test]
fn wrong_profile_reports_safe_named_case() {
    let error = profile::require(999).unwrap_err();
    assert_eq!(error.case, "profile.version");
    assert_eq!(error.category, FailureCategory::Profile);
}
#[test]
fn invalid_codec_reports_safe_named_case() {
    let error = field::codec::<Canonical>(&[], &[json!("SECRET_INPUT")]).unwrap_err();
    assert_eq!(error.case, "field.invalid");
    assert_eq!(error.category, FailureCategory::Assertion);
    assert!(!format!("{error:?} {error}").contains("SECRET_INPUT"));
}
#[test]
fn incorrect_canonical_output_is_rejected() {
    let error = field::codec(
        &[CodecCase {
            input: json!(" value "),
            canonical: json!("wrong"),
            expected: Canonical("value".into()),
        }],
        &[],
    )
    .unwrap_err();
    assert_eq!(error.case, "field.canonical");
}
