use authoring_builders::bon_style::ResourceConfig;
fn main() {
    let enabled = std::env::args().len() > 1;
    let mut builder = ResourceConfig::builder().name("sensor");
    if enabled { builder = builder.retries(5); }
    let _ = builder.build();
}
