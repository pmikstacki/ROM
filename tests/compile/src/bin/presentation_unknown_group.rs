use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "group")]
struct Example {
    #[resource(group = "missing")]
    name: String,
}
fn main() {}
