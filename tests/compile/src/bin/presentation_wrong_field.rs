use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "wrong-field")]
struct Example {
    #[resource(settings(group = "studio", label = "Studio"))]
    name: String,
}
fn main() {}
