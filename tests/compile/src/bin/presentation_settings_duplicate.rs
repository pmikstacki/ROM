use rom::Resource;
#[derive(Clone, Resource)]
#[resource(
    name = "settings",
    settings(group = "studio", label = "One"),
    settings(group = "studio", label = "Two")
)]
struct Example {
    name: String,
}
fn main() {}
