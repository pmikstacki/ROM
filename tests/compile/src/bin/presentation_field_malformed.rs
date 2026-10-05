use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "label")]
struct Example {
    #[resource(label = 7)]
    name: String,
}
fn main() {}
