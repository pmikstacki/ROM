use authoring_builders::typed::ResourceConfig;
fn main() { let _ = ResourceConfig::builder().name("sensor").retries("many").build(); }
