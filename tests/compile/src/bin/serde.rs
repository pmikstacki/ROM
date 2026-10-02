use rom::Resource;
#[derive(Clone,Resource)]
#[resource(name="bad",crate="::rom")]
struct Bad {
    #[serde(rename="different")]
    pub flag: bool,
}
fn main() {}
