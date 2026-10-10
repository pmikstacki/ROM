//! Trusted native-adapter support. These types are not public transport projections.
pub mod metadata {
    pub use crate::storage_state::metadata::{BundleDelta, StorageMetadata, prepare_bundle};
    pub use crate::storage_state::native_journal::{
        JournalDelta, JournalEntry, JournalImage, JournalRead, MetadataHeader, MetadataHeaderParts,
        NativeBundleDelta, journal_page, prepare_native_bundle,
    };
}
pub mod work {
    pub use crate::reaction_work::accounting::{
        EntryBytes, LedgerBytes, MapAccounting, WorkAccounting, entry_for_root, entry_for_work,
    };
    pub use crate::reaction_work::batch::validate_work_update_batch;
    pub use crate::reaction_work::claim_prefix::{claim_live, claim_prefix_accepts_source};
    pub use crate::reaction_work::incremental::{
        ReadFence, RootAccount, WorkDelta, WorkEdit, WorkHeader, WorkHeaderParts, WorkRead,
        prepare_enqueue, prepare_update,
    };
    pub use crate::reaction_work::incremental_bridge::WorkImage;
}
