//! Immutable run-scoped routing contracts and pure candidate selection.
mod cursor;
mod failover;
pub use failover::FailoverPolicy;
mod policy;
mod select;
pub use cursor::{RouteCursor, RoutingTier};
pub use policy::{CatalogModel, CatalogSnapshot, ModelPrice, RoutingPolicy, RunLimits, UsdNanos};
pub use select::{RouteDecision, choose, choose_continuation};
