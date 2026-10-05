use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "settings", settings(group = "studio"))]
struct Example {
    name: String,
}
fn main() {}
