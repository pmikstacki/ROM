use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "duplicate-field")]
struct Example {
    #[resource(label = "First", label = "Second")]
    name: String,
}
fn main() {}
