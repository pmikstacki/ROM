use crate::{Failure, args::Format};
use serde_json::Value;
use std::io::{ErrorKind, Write};
/// Escape terminal controls, including C1 controls which JSON permits literally.
pub fn emit(value: &Value, format: Format) -> Result<bool, Failure> {
    let encoded = match format {
        Format::Json => serde_json::to_string(value),
        Format::Human => serde_json::to_string_pretty(value),
    }
    .map_err(|_| Failure::transport("output could not be encoded"))?;
    let mut safe = String::with_capacity(encoded.len());
    for character in encoded.chars() {
        if character.is_control() && character != '\n' {
            use std::fmt::Write;
            write!(&mut safe, "\\u{:04x}", character as u32)
                .map_err(|_| Failure::transport("output could not be encoded"))?;
        } else {
            safe.push(character);
        }
    }
    safe.push('\n');
    let mut stdout = std::io::stdout().lock();
    let result = stdout
        .write_all(safe.as_bytes())
        .and_then(|()| stdout.flush());
    match result {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(false),
        Err(_) => Err(Failure::transport("output could not be written")),
    }
}
