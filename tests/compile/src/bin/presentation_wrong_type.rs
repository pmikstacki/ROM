use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "wrong-type", help = "Field only")]
struct Example {
    name: String,
}
fn main() {}
