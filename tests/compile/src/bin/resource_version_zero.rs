use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "invalid", version = 0)]
struct Invalid {
    value: bool,
}
fn main() {}
