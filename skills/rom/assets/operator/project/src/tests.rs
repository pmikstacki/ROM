use crate::request;

#[test]
fn submitted_request_is_a_valid_public_control_and_empty_identity_is_rejected() {
    let mut value = request();
    value.validate().unwrap();
    value.key.clear();
    assert!(value.validate().is_err());
}
