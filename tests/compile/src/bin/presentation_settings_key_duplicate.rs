use rom::Resource;
#[derive(Clone, Resource)]
#[resource(
    name = "settings",
    settings(group = "studio", group = "other", label = "Studio")
)]
struct Example {
    name: String,
}
fn main() {}
