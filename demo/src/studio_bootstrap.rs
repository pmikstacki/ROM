//! Explicit browser profile for the demo deployment.
use rom::Result;
use rom_studio_host::{BrowserStore, StudioBootstrap};

pub(super) fn profile(retry_epoch: u64) -> Result<StudioBootstrap> {
    const PAYLOAD_BYTES: usize = 1024 * 1024;
    const RECORD_BYTES: usize = PAYLOAD_BYTES + 4096;
    StudioBootstrap::new(
        "local",
        "rom-author-demo-v1",
        retry_epoch,
        PAYLOAD_BYTES,
        BrowserStore::new("rom-author-demo-intents-v1", RECORD_BYTES, 128, 10_000)?,
        BrowserStore::new("rom-author-demo-editors-v1", RECORD_BYTES, 128, 10_000)?,
    )
}
