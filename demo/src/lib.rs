//! A complete local application built from Resource declarations and business functions.
mod application;
mod host_signals;
mod identity;
mod model;
mod scratch;
mod startup;

pub use application::{Notices, build, declarations};
pub use identity::{bootstrap_actor, resolver, session_actor};
pub use model::{
    COMPLETE, DISPLAY, Dashboard, InventoryItem, NOTICE, Settings, StockCode, Task, rename_task,
};
pub use startup::bootstrap;

use identity::{domain, service};
use rom::{Command, Field, Resource, Runtime, Storage, Value};
use startup::seed;
use std::sync::Arc;

/// Finite, actual TCP smoke scenario shared by the command and integration test.
pub mod smoke;

/// Host-owned folder adapter and attachment lifecycle.
pub mod attachments;

/// Token-specific recovery from an explicit simulated business failure.
pub mod compensation;

/// Public-API reference journey through pending work recovery and live queries.
pub mod reference;

/// Offline schema upgrade and recovery of the complete reference application.
pub mod upgrade;

/// Authorized work recovery and explicit domain compensation through public APIs.
pub mod operator;

/// Host-owned local synthetic server and graceful lifecycle.
pub mod serving;

/// Opt-in host authentication from current provider configuration.
#[cfg(feature = "provider-profile")]
pub mod provider_profile;

/// Opt-in real Studio host with explicit local fixture configuration.
#[cfg(feature = "studio")]
pub mod studio;
#[cfg(feature = "studio")]
mod studio_application;
#[cfg(feature = "studio")]
mod studio_controls;
#[cfg(feature = "studio")]
mod studio_model;
#[cfg(feature = "studio")]
mod studio_startup;

#[cfg(feature = "studio")]
mod studio_blobs;

#[cfg(feature = "studio")]
mod studio_profile;
#[cfg(all(feature = "studio", test))]
mod studio_profile_tests;

#[cfg(any(feature = "studio", feature = "provider-profile"))]
mod host_files;
#[cfg(all(test, any(feature = "studio", feature = "provider-profile")))]
mod host_files_tests;

#[cfg(all(feature = "studio", test))]
mod studio_model_tests;
