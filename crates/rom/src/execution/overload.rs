//! Host-only fixed boundary evidence; no intake or authorization side effects.
use super::Runtime;
use crate::CoreOverloadStats;

impl Runtime {
    /// Return advisory totals for this Runtime's four core overload boundaries.
    /// Cloned handles share the counts. Complete all caller/background producers
    /// and drain owned work before claiming exact terminal counts with no overflow.
    /// Background work
    /// and streams are included; this is not per-request attribution.
    pub fn core_overload_stats(&self) -> CoreOverloadStats {
        self.0.core_overloads.snapshot()
    }
}
