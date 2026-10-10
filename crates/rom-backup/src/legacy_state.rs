//! Explicit conversion of state written before operator recovery metadata.
use rom::{Error, Result, StorageState};
use serde_json::{Value, json};

/// Decode pre-format-8 native state without accepting newer metadata in an old format.
/// Existing fields are retained; revisions, profiles and the operator ledger are initialized.
pub fn decode_legacy_storage_state(mut value: Value) -> Result<StorageState> {
    reject_scheduling_fields(&value)?;
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

/// Earlier native markers cannot represent a frozen scheduling floor.
pub(crate) fn reject_scheduling_fields(value: &Value) -> Result<()> {
    let records = value
        .get("work")
        .and_then(|ledger| ledger.get("work"))
        .and_then(Value::as_object)
        .ok_or(Error::Storage)?;
    if records.values().any(|record| {
        record
            .get("pending")
            .and_then(Value::as_object)
            .is_some_and(|pending| pending.contains_key("not_before"))
    }) {
        return Err(Error::Storage);
    }
    Ok(())
}

/// Validate only frozen Intent metadata, not application payload field names.
pub(crate) fn reject_intent_scheduling_fields(value: &Value) -> Result<()> {
    if value
        .as_object()
        .ok_or(Error::Storage)?
        .contains_key("not_before")
    {
        return Err(Error::Storage);
    }
    Ok(())
}

pub(crate) fn reject_snapshot_scheduling_fields(value: &Value) -> Result<()> {
    reject_scheduling_fields(value.get("state").ok_or(Error::Storage)?)?;
    for effect in value
        .get("effects")
        .and_then(Value::as_array)
        .ok_or(Error::Storage)?
    {
        reject_intent_scheduling_fields(effect.get("intent").ok_or(Error::Storage)?)?;
    }
    Ok(())
}
