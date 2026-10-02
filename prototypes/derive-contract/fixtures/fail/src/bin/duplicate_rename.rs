use resource_contract::Resource;
#[derive(Resource)]
#[resource(name = "broken")]
struct Broken {
    #[resource(rename = "a", rename = "b")] // error-site
    value: String,
}
fn main() {}
