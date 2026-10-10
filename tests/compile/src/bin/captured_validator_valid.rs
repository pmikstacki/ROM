use rom::{Actor, Resource, Result};
#[derive(Clone, Resource)]
#[resource(name = "captured-validator-records")]
struct Record {
    amount: u64,
}
fn validate(_: &Actor, _: Option<&Record>, _: Option<&Record>) -> Result<()> {
    Ok(())
}
fn main() {
    let pointer: fn(&Actor, Option<&Record>, Option<&Record>) -> Result<()> = validate;
    let _named = Record::definition().validate_transition(validate);
    let _pointer = Record::definition().validate_transition(pointer);
    let _closure = Record::definition().validate_transition(|_, _, _| Ok(()));
    let catalog = std::sync::Arc::new(4_u64);
    let _captured = Record::definition().validate_transition(move |_, _, after| {
        if after.is_some_and(|record| record.amount > *catalog) {
            return Err(rom::Error::Denied);
        }
        Ok(())
    });
}
