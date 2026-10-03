use rom::Input;
#[derive(Clone, Input)]
struct Payload {
    #[input(rename = "first", rename = "second")]
    value: String,
}
fn main() {}
