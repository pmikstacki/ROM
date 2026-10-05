use crate::scalar::{invalid, string_field};
string_field!(
    Color,
    "rom.color",
    color,
    "sRGB hexadecimal color #RRGGBB or #RRGGBBAA; lowercase canonical encoding."
);
string_field!(
    Email,
    "rom.email",
    email,
    "ASCII dot-atom mailbox with DNS labels; local case is preserved and domain case is folded."
);
string_field!(
    Url,
    "rom.url",
    url,
    "Absolute HTTP or HTTPS URL without credentials; parsed without network access."
);
string_field!(
    Multiline,
    "rom.multiline",
    multiline,
    "Exact UTF-8 text up to one MiB, including empty text and line endings."
);
string_field!(
    JsonDocument,
    "rom.json-document",
    json_document,
    "Validated JSON text up to one MiB; exact lexical encoding is preserved."
);
fn color(value: &str) -> rom::Result<String> {
    if !matches!(value.len(), 7 | 9)
        || !value.starts_with('#')
        || !value[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(invalid("rom.color"));
    }
    Ok(value.to_ascii_lowercase())
}
fn email(value: &str) -> rom::Result<String> {
    let (local, domain) = value.split_once('@').ok_or_else(|| invalid("rom.email"))?;
    let atom = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-/=?^_`{|}~".contains(&b))
    };
    let label = |part: &str| {
        !part.is_empty()
            && part.len() <= 63
            && part.as_bytes()[0].is_ascii_alphanumeric()
            && part.as_bytes()[part.len() - 1].is_ascii_alphanumeric()
            && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    if value.len() > 254
        || local.len() > 64
        || domain.len() > 253
        || !local.split('.').all(atom)
        || !domain.split('.').all(label)
    {
        return Err(invalid("rom.email"));
    }
    Ok(format!("{local}@{}", domain.to_ascii_lowercase()))
}
fn url(value: &str) -> rom::Result<String> {
    if value.len() > 8192
        || value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
    {
        return Err(invalid("rom.url"));
    }
    let parsed = url::Url::parse(value).map_err(|_| invalid("rom.url"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || !value.contains("://")
    {
        return Err(invalid("rom.url"));
    }
    Ok(parsed.into())
}
fn multiline(value: &str) -> rom::Result<String> {
    if value.len() > 1_048_576 {
        return Err(invalid("rom.multiline"));
    }
    Ok(value.into())
}
fn json_document(value: &str) -> rom::Result<String> {
    if value.len() > 1_048_576 {
        return Err(invalid("rom.json-document"));
    }
    serde_json::from_str::<Box<serde_json::value::RawValue>>(value)
        .map_err(|_| invalid("rom.json-document"))?;
    Ok(value.into())
}
