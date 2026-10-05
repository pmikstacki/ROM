#![allow(non_camel_case_types)]
use rom::Resource;
type bool = String;
#[derive(Clone, Resource)]
#[resource(name = "shadowed-builtin", title_field = "title")]
struct Example {
    title: bool,
}
fn main() {
    assert_eq!(Example::descriptor().fields[0].shape, rom::Shape::String);
    assert_eq!(
        Example::presentation().unwrap().title_field.as_deref(),
        Some("title")
    );
}
