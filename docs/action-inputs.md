# Structured action inputs

An action can accept a named payload without a handwritten codec:

```rust
use rom::{Action, Input, Resource};

#[derive(Clone, Resource)]
#[resource(name = "stock")]
struct Stock { available: u64 }

#[derive(Clone, Input)]
struct ReserveInput {
    #[input(rename = "reservation-token")]
    token: String,
    quantity: u64,
}

const RESERVE: Action<Stock, ReserveInput> = Action::new("reserve", |stock, input| {
    stock.available = stock.available.checked_sub(input.quantity)
        .ok_or_else(|| rom::Error::invalid("stock", "quantity"))?;
    // An application can use input.token to record its reservation identity.
    let _ = input.token;
    Ok(vec![])
});
```

Register `RESERVE` with `Stock::definition().action(RESERVE)`. Supply `ReserveInput` to `Command::action`. Actions still need the normal Resource and field policies. The payload is a command argument, not another Resource.

Each member uses its `Field` codec. The codec rejects omitted required members, unknown keys and invalid values. `Option<T>` is a required nullable member. `Presence<T>` allows omission, and `Presence<Option<T>>` also permits null. Structured members use normal object omission/null. A standalone `Presence<T>` action argument retains its existing tagged envelope.

`#[input(rename = "wire-name")]` applies to both encoding and decoding, and direct codec errors name that wire field. The runtime reports the Resource kind and `action.wire-field` for a declared field. Other decoding errors remain sanitized to the action name. A renamed Cargo dependency can use `#[input(crate = "::framework")]`. The default facade path is `::rom`. Custom Field codecs participate directly.

Only concrete named-field structs are supported. The derive implements Input, not Field. It does not add nested object shapes, Resource registration, action metadata/discovery schemas, generic payload definitions or independent Serde attributes. A type should not implement Field and also derive Input because Field already supplies the blanket Input implementation. Duplicate raw JSON keys must be rejected before conversion to Value by the transport decoder.
