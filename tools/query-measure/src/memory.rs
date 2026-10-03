//! Optional per-query Rust allocator scope, separate from latency trials.
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Heap {
    pub allocations: u64,
    pub total_allocated_bytes: u64,
    pub peak_tracked_bytes: usize,
    pub tracked_bytes_at_return: usize,
}

pub(crate) struct Profile {
    #[cfg(feature = "heap")]
    profiler: Option<dhat::Profiler>,
}
impl Profile {
    pub(crate) fn begin(enabled: bool) -> Self {
        #[cfg(feature = "heap")]
        {
            Self {
                profiler: enabled.then(|| dhat::Profiler::builder().testing().build()),
            }
        }
        #[cfg(not(feature = "heap"))]
        {
            let _ = enabled;
            Self {}
        }
    }
    pub(crate) fn finish(self) -> Option<Heap> {
        #[cfg(feature = "heap")]
        {
            self.profiler.as_ref().map(|_| {
                let stats = dhat::HeapStats::get();
                Heap {
                    allocations: stats.total_blocks,
                    total_allocated_bytes: stats.total_bytes,
                    peak_tracked_bytes: stats.max_bytes,
                    tracked_bytes_at_return: stats.curr_bytes,
                }
            })
        }
        #[cfg(not(feature = "heap"))]
        {
            None
        }
    }
}
