use authoring_builders::manual::ResourceConfig;
fn main() { let _ = ResourceConfig::builder().name("sensor").retries("many").build(); }
