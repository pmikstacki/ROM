use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "invalid", version = "2")]
struct Invalid {
    value: bool,
}
fn main() {}
