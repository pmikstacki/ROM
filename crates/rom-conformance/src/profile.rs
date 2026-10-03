use crate::{ConformanceError, ConformanceResult, FailureCategory};
pub const PROFILE_VERSION: u32 = 1;
pub fn require(version: u32) -> ConformanceResult {
    if version == PROFILE_VERSION {
        Ok(())
    } else {
        Err(ConformanceError::new(
            "profile.version",
            FailureCategory::Profile,
        ))
    }
}
