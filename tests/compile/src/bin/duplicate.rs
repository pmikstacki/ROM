use rom::Resource;
#[derive(Clone,Resource)]
#[resource(name="bad",crate="::rom")]
struct Bad {
    pub flag: bool,
    #[resource(rename="flag")]
    pub other: bool,
}
fn main() {}
