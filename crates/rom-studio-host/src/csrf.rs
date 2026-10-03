use axum::http::HeaderMap;
use rom::{Error, Result};

pub(crate) fn check_origin(headers: &HeaderMap, expected: &str) -> Result<()> {
    let mut origins = headers.get_all("origin").iter();
    let origin = origins
        .next()
        .ok_or(Error::Denied)?
        .to_str()
        .map_err(|_| Error::Denied)?;
    if origins.next().is_some() || origin != expected {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) fn session_cookie(headers: &HeaderMap, name: &str) -> Result<String> {
    let mut selected = None;
    for header in headers.get_all("cookie") {
        let raw = header.to_str().map_err(|_| Error::Denied)?;
        if raw.len() > 8192 {
            return Err(Error::TooLarge);
        }
        for cookie in raw.split(';') {
            if let Some((key, value)) = cookie.trim().split_once('=')
                && key == name
            {
                if selected.is_some() || value.is_empty() {
                    return Err(Error::Denied);
                }
                selected = Some(value.to_owned());
            }
        }
    }
    selected.ok_or(Error::Denied)
}

pub(crate) fn check_token(headers: &HeaderMap, session: &crate::session::Session) -> Result<()> {
    let mut values = headers.get_all("x-rom-csrf").iter();
    if !values
        .next()
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| session.check_csrf(value))
        || values.next().is_some()
    {
        return Err(Error::Denied);
    }
    Ok(())
}
