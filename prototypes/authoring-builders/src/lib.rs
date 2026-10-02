//! Disposable authoring comparison, not ROM's public API.
#[cfg(feature = "bon")]
pub mod bon_style;
#[cfg(feature = "manual")]
pub mod manual;
#[cfg(feature = "typed")]
pub mod typed;
