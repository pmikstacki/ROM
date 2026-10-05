use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "renamed", title_field = "name")]
struct Example {
    #[resource(rename = "display-name")]
    name: String,
}
fn main() {}
