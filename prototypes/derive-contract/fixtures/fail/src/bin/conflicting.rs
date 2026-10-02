use resource_contract::Resource;
#[derive(Resource)]
#[resource(name = "first")]
#[resource(name = "second")] // error-site
struct Broken {
    enabled: bool,
}
fn main() {}
