#[derive(typed_builder::TypedBuilder)]
struct Input {
    #[builder(default, setter(strip_option))]
    note: Option<String>,
}
fn main() { let _ = Input::builder().note(None).build(); }
