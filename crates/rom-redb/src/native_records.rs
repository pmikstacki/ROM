//! Bounded decoding of public native lookups with exact persisted identities.
use crate::native_journal::Budget;
use rom::{Error, Receipt, Result, Row};

pub(super) fn row(key: (&str, &str), raw: &str, budget: Option<&Budget>) -> Result<Row> {
    admit(&[key.0, key.1, raw], budget)?;
    let row: Row = serde_json::from_str(raw).map_err(|_| Error::Storage)?;
    if row.key.kind != key.0 || row.key.id != key.1 {
        return Err(Error::Storage);
    }
    Ok(row)
}

pub(super) fn receipt(identity: &str, raw: &str, budget: Option<&Budget>) -> Result<Receipt> {
    admit(&[identity, raw], budget)?;
    let receipt: Receipt = serde_json::from_str(raw).map_err(|_| Error::Storage)?;
    if receipt.identity != identity {
        return Err(Error::Storage);
    }
    Ok(receipt)
}

fn admit(parts: &[&str], budget: Option<&Budget>) -> Result<()> {
    if let Some(budget) = budget {
        let bytes = parts.iter().try_fold(0_usize, |total, part| {
            total.checked_add(part.len()).ok_or(Error::TooLarge)
        })?;
        budget.charge(bytes)?;
    }
    Ok(())
}
