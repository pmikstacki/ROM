//! Experimental facade: ordinary traits always available; derive is optional.
pub use resource_contract_core::*;
#[cfg(feature = "derive")]
pub use resource_contract_derive::Resource;
