use rom::Input;
#[derive(Clone, Input)]
struct Payload {
    #[serde(default)]
    value: String,
}
fn main() {}
