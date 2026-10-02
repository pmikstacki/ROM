use authoring_builders::manual::ResourceConfig;
fn main() { assert_eq!(ResourceConfig::builder().name("first").name("second").build().unwrap().name, "second"); }
