use rom::Input;
#[derive(Clone, Input)]
struct Payload {
    value: String,
    #[input(rename = "value")]
    other: String,
}
fn main() {}
