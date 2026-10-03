extern crate rom as framework;
use framework::Input;
#[derive(Clone, Input)]
#[input(crate = "::framework")]
struct Payload {
    #[input(rename = "value-with-alias")]
    value: bool,
}
fn main() {
    let metadata = Payload::descriptor().unwrap();
    let framework::InputDescriptor::Object(fields) = metadata else {
        panic!("object expected")
    };
    assert_eq!(fields[0].name, "value-with-alias");
    assert_eq!(fields[0].shape, framework::Shape::Bool);
    let value = Payload { value: false };
    assert_eq!(
        value.encode(),
        framework::json!({"value-with-alias": false})
    );
    assert!(!Payload::decode(value.encode()).unwrap().value);
}
