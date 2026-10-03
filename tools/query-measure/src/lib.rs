//! Maintained Runtime query and write measurements; not a published application API.
#[cfg(feature = "queries")]
mod cases;
pub mod fixture;
#[cfg(feature = "queries")]
mod memory;
#[cfg(feature = "queries")]
mod observed;
#[cfg(feature = "queries")]
pub mod run;
#[cfg(feature = "queries")]
mod settings;
mod space;
pub mod write;
pub mod write_run;

pub type Failure = Box<dyn std::error::Error + Send + Sync>;

#[cfg(test)]
mod tests;
