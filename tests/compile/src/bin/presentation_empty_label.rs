use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "empty", label = "")]
struct Example {
    name: String,
}
fn main() {}
