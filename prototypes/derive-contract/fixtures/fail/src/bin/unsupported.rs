use resource_contract::Resource;
struct Unsupported;
#[derive(Resource)]
#[resource(name = "broken")]
struct Broken {
    payload: Unsupported, // error-site
}
fn main() {}
