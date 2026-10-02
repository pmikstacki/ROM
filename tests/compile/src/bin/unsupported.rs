use rom::Resource;
#[derive(Clone,Resource)]
#[resource(name="unsupported",crate="::rom")]
struct Bad {
    pub unsupported: f32,
}
fn main() {}
