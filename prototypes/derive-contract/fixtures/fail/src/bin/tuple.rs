use resource_contract::Resource;
#[derive(Resource)]
#[resource(name = "broken")]
struct Broken(bool); // error-site
fn main() {}
