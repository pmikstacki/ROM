use crate::scalar::{invalid, string_field};
string_field!(
    Decimal,
    "rom.decimal",
    decimal,
    "Exact decimal string, at most 1024 bytes, without exponent notation or floating-point conversion."
);
fn decimal(value: &str) -> rom::Result<String> {
    if value.len() > 1024 {
        return Err(invalid("rom.decimal"));
    }
    let (negative, unsigned) = value
        .strip_prefix('-')
        .map_or((false, value), |v| (true, v));
    let (integer, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(a, b)| (a, Some(b)));
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(integer) || fraction.is_some_and(|v| !digits(v)) {
        return Err(invalid("rom.decimal"));
    }
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let fraction = fraction.unwrap_or("").trim_end_matches('0');
    let sign = if negative && (integer != "0" || !fraction.is_empty()) {
        "-"
    } else {
        ""
    };
    Ok(if fraction.is_empty() {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}.{fraction}")
    })
}
