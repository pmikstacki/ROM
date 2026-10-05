use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "unknown", format = "email")]
struct Example {
    name: String,
}
fn main() {}
