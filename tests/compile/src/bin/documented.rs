//! A fully documented resource author can require documentation from generated APIs.
#![deny(missing_docs)]
use rom::Resource;
/// An ordinary resource.
#[derive(Clone, Resource)]
#[resource(name="documented",crate="::rom")]
pub struct Documented {
    /// Whether the operation has completed.
    pub done: bool,
}
fn main() { let _query=Documented::done_field().equals(false); }
