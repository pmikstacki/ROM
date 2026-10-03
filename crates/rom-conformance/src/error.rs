use crate::PROFILE_VERSION;
use std::fmt;

/// Static failure categories never retain arbitrary callback errors or values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureCategory {
    Profile,
    Adapter,
    Assertion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConformanceError {
    pub profile: u32,
    pub case: &'static str,
    pub category: FailureCategory,
}
pub type ConformanceResult = Result<(), ConformanceError>;
impl ConformanceError {
    pub(crate) fn new(case: &'static str, category: FailureCategory) -> Self {
        Self {
            profile: PROFILE_VERSION,
            case,
            category,
        }
    }
}
impl fmt::Display for ConformanceError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "native profile {}: {} ({:?})",
            self.profile, self.case, self.category
        )
    }
}
impl std::error::Error for ConformanceError {}
pub(crate) fn check(ok: bool, case: &'static str) -> ConformanceResult {
    if ok {
        Ok(())
    } else {
        Err(ConformanceError::new(case, FailureCategory::Assertion))
    }
}
pub(crate) fn observe<T, E>(
    result: Result<T, E>,
    case: &'static str,
) -> Result<T, ConformanceError> {
    result.map_err(|_| ConformanceError::new(case, FailureCategory::Adapter))
}
