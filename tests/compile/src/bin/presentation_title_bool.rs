use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "boolean", title_field = "enabled")]
struct Example {
    enabled: ::core::primitive::bool,
}
fn main() {}
