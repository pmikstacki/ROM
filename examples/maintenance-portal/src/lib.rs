//! Independent application composed exclusively through ROM's public interfaces.
#![forbid(unsafe_code)]
mod application;
mod catalog;
mod layout;
mod policy;
mod resources;
mod work;

pub use application::declarations;
pub use catalog::{MaintenanceGuide, public_guest};
pub use layout::WorkspaceLayout;
pub use resources::{Equipment, Inspection, PortalSettings};
pub use work::{COMPLETE, WorkOrder};
