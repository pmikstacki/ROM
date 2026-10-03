use rom as renamed;
use rom::{Resource, json};
#[derive(Clone, Resource)]
#[resource(name = "default-version")]
struct DefaultVersion {
    label: String,
}
#[derive(Clone, Resource)]
#[resource(name = "versioned", version = 2, crate = "renamed")]
struct Versioned {
    #[resource(rename = "display-name")]
    label: String,
}
#[derive(Clone, Resource)]
#[resource(name = "maximum-version", version = 4294967295)]
struct MaximumVersion {
    enabled: bool,
}
fn main() {
    assert_eq!(DefaultVersion::descriptor().version, 1);
    assert_eq!(Versioned::descriptor().version, 2);
    assert_eq!(MaximumVersion::descriptor().version, u32::MAX);
    let descriptor = Versioned::descriptor();
    assert_eq!(descriptor.kind, "versioned");
    assert_eq!(descriptor.fields[0].name, "display-name");
    let value = json!({"display-name":"unchanged codec"});
    let decoded = Versioned::decode(value.clone()).unwrap();
    assert_eq!(decoded.encode(), value);
    assert_eq!(decoded.label, "unchanged codec");
}
