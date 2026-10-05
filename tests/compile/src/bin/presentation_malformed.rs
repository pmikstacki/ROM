use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "malformed", settings = "studio")]
struct Example {
    name: String,
}
fn main() {}
