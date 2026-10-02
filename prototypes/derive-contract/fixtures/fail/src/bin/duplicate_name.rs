use resource_contract::Resource;
#[derive(Resource)]
#[resource(name = "broken")]
struct Broken {
    #[resource(rename = "same")]
    first: bool,
    #[resource(rename = "same")] // error-site
    second: bool,
}
fn main() {}
