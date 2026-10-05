use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "duplicate", label = "First", label = "Second")]
struct Example {
    name: String,
}
fn main() {}
