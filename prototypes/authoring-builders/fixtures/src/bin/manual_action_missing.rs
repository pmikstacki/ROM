use authoring_builders::manual::{ActionInput, MissingField};
fn main() { assert_eq!(ActionInput::builder().resource_id("s1").build(), Err(MissingField("expected_revision"))); }
