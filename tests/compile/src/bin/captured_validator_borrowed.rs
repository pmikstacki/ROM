use rom::Resource;
#[derive(Clone, Resource)]
#[resource(name = "captured-validator-records")]
struct Record {
    amount: u64,
}
fn main() {
    let catalog = 4_u64;
    let _definition = Record::definition().validate_transition(|_, _, after| {
        if after.is_some_and(|record| record.amount > catalog) {
            return Err(rom::Error::Denied);
        }
        Ok(())
    });
}
