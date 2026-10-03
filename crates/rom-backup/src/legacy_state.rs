//! Explicit conversion of state written before operator recovery metadata.
use rom::{Error, Result, StorageState};
use serde_json::{Value, json};

/// Decode pre-format-8 native state without accepting newer metadata in an old format.
/// Existing fields are retained; revisions, profiles and the operator ledger are initialized.
pub fn decode_legacy_storage_state(mut value: Value) -> Result<StorageState> {
    upgrade_state(&mut value)?;
    serde_json::from_value(value).map_err(|_| Error::Storage)
}

pub(crate) fn upgrade_state(value: &mut Value) -> Result<()> {
    let state = value.as_object_mut().ok_or(Error::Storage)?;
    if state.contains_key("operator") {
        return Err(Error::Storage);
    }
    let work = state
        .get_mut("work")
        .and_then(|ledger| ledger.get_mut("work"))
        .and_then(Value::as_object_mut)
        .ok_or(Error::Storage)?;
    for value in work.values_mut() {
        let record = value.as_object_mut().ok_or(Error::Storage)?;
        if record.contains_key("revision")
            || record.get("state") == Some(&json!("AwaitingReconciliation"))
            || record.get("state").is_some_and(|state| {
                state
                    .as_object()
                    .is_some_and(|state| state.contains_key("AwaitingReconciliation"))
            })
        {
            return Err(Error::Storage);
        }
        let pending = record
            .get_mut("pending")
            .and_then(Value::as_object_mut)
            .ok_or(Error::Storage)?;
        if pending.contains_key("delivery_profile") {
            return Err(Error::Storage);
        }
        pending.insert("delivery_profile".into(), json!("AtLeastOnce"));
        record.insert("revision".into(), json!(0));
    }
    state.insert(
        "operator".into(),
        serde_json::to_value(rom::OperatorLedger::default()).map_err(|_| Error::Storage)?,
    );
    Ok(())
}
