use rom::*;
use rom_backup::{MigrationPlan, ResourceMigration};
#[derive(Clone, Debug, Resource)]
#[resource(name = "studio-resilience-records")]
pub struct Record {
    pub owner: String,
    pub value: u64,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "studio-resilience-records", version = 2)]
pub struct RecordV2 {
    pub owner: String,
    pub total: u64,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "studio-resilience-ledgers")]
pub struct Ledger {
    pub owner: String,
    pub total: u64,
}
pub fn conversion(record: Record) -> Result<RecordV2> {
    Ok(RecordV2 {
        owner: record.owner,
        total: record.value,
    })
}
pub fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Record, RecordV2>(conversion).unwrap(),
    ])
    .unwrap()
    .validate_work(|work| {
        if work.delivery_profile != DeliveryProfile::ReconcileBeforeRetry
            || work.definition != "notice"
            || work.version != 1
        {
            return Err(Error::Storage);
        }
        let WorkPayload::Notification { source, payload } = &work.payload else {
            return Err(Error::Storage);
        };
        RecordV2::decode(source.value.clone().ok_or(Error::Storage)?)?;
        if payload != &json!("unchanged delivery") {
            return Err(Error::Storage);
        }
        Ok(())
    })
}
