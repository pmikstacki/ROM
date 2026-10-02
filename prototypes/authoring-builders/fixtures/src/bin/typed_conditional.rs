use authoring_builders::typed::ResourceConfig;
fn main() {
    let enabled = std::env::args().len() > 1;
    let mut builder = ResourceConfig::builder().name("sensor");
    if enabled { builder = builder.retries(5); }
    let _ = builder.build();
}
