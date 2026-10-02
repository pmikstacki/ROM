use authoring_builders::bon_style::ResourceConfig;
fn main() { let _ = ResourceConfig::builder().name("sensor").retries("many").build(); }
