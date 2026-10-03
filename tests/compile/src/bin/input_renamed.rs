extern crate rom as framework;
use framework::Input;
#[derive(Clone, Input)]
#[input(crate = "::framework")]
struct Payload {
    #[input(rename = "value-with-alias")]
    value: bool,
}
fn main() {
    let value = Payload { value: false };
    assert_eq!(value.encode(), framework::json!({"value-with-alias": false}));
    assert!(!Payload::decode(value.encode()).unwrap().value);
}
