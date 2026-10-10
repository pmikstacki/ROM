//! Install-time metadata only; a name never grants invocation authority.
pub fn action_route<R: rom::Resource, I: rom::Input>(action: rom::Action<R, I>) -> &'static str {
    action.name()
}
fn main() {}
