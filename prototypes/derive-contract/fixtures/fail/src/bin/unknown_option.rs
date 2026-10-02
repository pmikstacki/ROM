use resource_contract::Resource;
#[derive(Resource)]
#[resource(name = "broken")]
struct Broken {
    #[resource(nullable = true)] // error-site
    value: String,
}
fn main() {}
