#[cfg(any(feature = "jwt", feature = "introspection"))]
pub(crate) fn valid_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048 && value.trim() == value
}

// Absence defaults to None; a present claim must deserialize as T. In particular,
// null does not silently disable optional time checks or proof requirements.
#[cfg(any(feature = "jwt", feature = "introspection"))]
pub(crate) fn present_claim<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
