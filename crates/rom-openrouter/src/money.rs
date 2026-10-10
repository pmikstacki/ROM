//! Checked decimal conversion from original wire lexemes; never binary floating point.
use rom_ai::{AiError, AiResult, UsdNanos};

pub(crate) fn scaled(decimal: &str, scale: i32) -> AiResult<UsdNanos> {
    if decimal.is_empty() || decimal.len() > 128 || decimal.starts_with('-') {
        return Err(AiError::InvalidOutput);
    }
    let (mantissa, exponent) = match decimal.find(['e', 'E']) {
        Some(index) => {
            let exponent = decimal[index + 1..]
                .parse::<i32>()
                .map_err(|_| AiError::InvalidOutput)?;
            if exponent.abs_diff(0) > 10_000 {
                return Err(AiError::InvalidOutput);
            }
            (&decimal[..index], exponent)
        }
        None => (decimal, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || (whole.len() > 1 && whole.starts_with('0'))
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || (mantissa.contains('.') && fraction.is_empty())
    {
        return Err(AiError::InvalidOutput);
    }
    let digits = format!("{whole}{fraction}");
    if digits.bytes().all(|byte| byte == b'0') {
        return Ok(UsdNanos(0));
    }
    let shift = exponent
        .checked_add(scale)
        .and_then(|value| value.checked_sub(fraction.len() as i32))
        .ok_or(AiError::InvalidOutput)?;
    let (integer, remainder, zeros) = if shift >= 0 {
        (digits.as_str(), false, shift as usize)
    } else {
        let split = digits.len().saturating_sub((-shift) as usize);
        (
            &digits[..split],
            digits[split..].bytes().any(|byte| byte != b'0'),
            0,
        )
    };
    let integer = integer.trim_start_matches('0');
    if integer
        .len()
        .checked_add(zeros)
        .is_none_or(|length| length > 20)
    {
        return Err(AiError::InvalidOutput);
    }
    let mut amount = 0_u64;
    for byte in integer.bytes() {
        amount = amount
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            .ok_or(AiError::InvalidOutput)?;
    }
    for _ in 0..zeros {
        amount = amount.checked_mul(10).ok_or(AiError::InvalidOutput)?;
    }
    if remainder {
        amount = amount.checked_add(1).ok_or(AiError::InvalidOutput)?;
    }
    Ok(UsdNanos(amount))
}
pub(crate) fn wire_usd(amount: UsdNanos) -> AiResult<Box<serde_json::value::RawValue>> {
    let whole = amount.0 / 1_000_000_000;
    let fraction = amount.0 % 1_000_000_000;
    let decimal = if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:09}")
            .trim_end_matches('0')
            .to_owned()
    };
    serde_json::value::RawValue::from_string(decimal).map_err(|_| AiError::InvalidRequest)
}
