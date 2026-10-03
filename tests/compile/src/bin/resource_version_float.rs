use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "invalid", version = 2.5)]
struct Invalid {
    value: bool,
}
fn main() {}
