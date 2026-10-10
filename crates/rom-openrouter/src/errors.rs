//! Sanitized errors and nonacceptance knowledge.
use rom_ai::{AiError, AiResult};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn now_ms() -> AiResult<u64> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AiError::DeadlineExceeded)?
            .as_millis(),
    )
    .map_err(|_| AiError::DeadlineExceeded)
}
pub(crate) fn retry_floor(header: Option<&str>, now: u64) -> AiResult<u64> {
    let Some(header) = header else {
        return Ok(1000);
    };
    if header.len() > 128 {
        return Err(AiError::InvalidOutput);
    }
    let floor = if !header.is_empty() && header.bytes().all(|byte| byte.is_ascii_digit()) {
        header
            .parse::<u64>()
            .ok()
            .and_then(|seconds| seconds.checked_mul(1000))
            .ok_or(AiError::InvalidOutput)?
    } else {
        let date = httpdate::parse_http_date(header).map_err(|_| AiError::InvalidOutput)?;
        let date = u64::try_from(
            date.duration_since(UNIX_EPOCH)
                .map_err(|_| AiError::InvalidOutput)?
                .as_millis(),
        )
        .map_err(|_| AiError::InvalidOutput)?;
        date.saturating_sub(now).max(1)
    };
    if !(1..=300_000).contains(&floor) {
        return Err(AiError::InvalidOutput);
    }
    Ok(floor)
}
pub(crate) fn http_error(status: u16) -> AiError {
    match status {
        400 => AiError::InvalidRequest,
        401 | 403 => AiError::Denied,
        402 => AiError::BudgetExhausted,
        404 | 502 | 503 => AiError::ProviderUnavailable,
        _ => AiError::UnknownOutcome,
    }
}
