//! A complete local application built from Resource declarations and business functions.
mod application;
mod identity;
mod model;
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
