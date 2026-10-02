use authoring_builders::manual::{MissingField, ResourceConfig};
fn main() { assert_eq!(ResourceConfig::builder().build(), Err(MissingField("name"))); }
