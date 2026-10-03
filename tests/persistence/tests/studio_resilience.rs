//! Native resilience regressions; generated identity signatures are not provider/browser certification.
#[path = "support/child_process.rs"]
#[allow(dead_code)] // This shared helper also supports readiness and forced-exit fixtures.
mod child_process;
#[path = "../../../crates/rom-auth/tests/support/signing.rs"]
mod signing;
#[path = "studio_resilience/mod.rs"]
mod studio_resilience;
