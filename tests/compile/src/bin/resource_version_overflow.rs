use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "invalid", version = 4294967296)]
struct Invalid {
    value: bool,
}
fn main() {}
