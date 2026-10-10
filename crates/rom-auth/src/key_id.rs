// A ROM profile bound, not a JOSE length requirement. Preserve opaque UTF-8 exactly.
const MAX_BYTES: usize = 256;

/// Check ROM's opaque JOSE key-ID profile: nonempty, at most 256 UTF-8 bytes,
/// and no Unicode control characters. This does not establish trust in a key.
/// The value is matched exactly, without case folding or Unicode normalization.
pub fn valid_key_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_BYTES && !value.chars().any(char::is_control)
}
