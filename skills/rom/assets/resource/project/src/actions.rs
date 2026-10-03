use crate::Inventory;
use rom::{Action, Error, Intent, Result};

fn reserve(inventory: &mut Inventory, quantity: u64) -> Result<Vec<Intent>> {
    if quantity == 0 || quantity > inventory.available {
        return Err(Error::Denied);
    }
    inventory.available -= quantity;
    Ok(vec![])
}
pub const RESERVE: Action<Inventory, u64> = Action::new("reserve", reserve);
