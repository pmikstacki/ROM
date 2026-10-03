//! Typed schema admission after the shared duplicate-free decoder.
pub(crate) fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> rom::Result<T> {
    let value = rom::parse_json(bytes)?;
    serde_json::from_value(value).map_err(|_| rom::Error::invalid("request", "schema"))
}
