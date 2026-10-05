use rom::Resource;
#[derive(Clone, Resource)]
#[resource(
    name = "group",
    group(name = "same", label = "One"),
    group(name = "same", label = "Two")
)]
struct Example {
    name: String,
}
fn main() {}
