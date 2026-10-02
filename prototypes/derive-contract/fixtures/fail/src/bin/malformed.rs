use resource_contract::Resource;
#[derive(Resource)]
#[resource(name = 42)] // error-site
struct Broken {
    enabled: bool,
}
fn main() {}
