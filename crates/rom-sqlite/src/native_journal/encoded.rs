//! Bundle payload encoding completes before the first Resource publication.
use rom::{Bundle, Error, Result};

pub(crate) struct EncodedBundle {
    pub(crate) row: String,
    pub(crate) receipt: String,
    pub(crate) effects: Vec<String>,
}
impl EncodedBundle {
    pub(crate) fn new(bundle: &Bundle) -> Result<Self> {
        let row = serde_json::to_string(&bundle.receipt.row).map_err(|_| Error::Storage)?;
        let receipt = serde_json::to_string(&bundle.receipt).map_err(|_| Error::Storage)?;
        let effects = bundle
            .effects
            .iter()
            .map(|e| serde_json::to_string(e).map_err(|_| Error::Storage))
            .collect::<Result<_>>()?;
        Ok(Self {
            row,
            receipt,
            effects,
        })
    }
}
