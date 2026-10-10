//! Frozen application opt-in; never proof that a provider attempt was not accepted.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailoverPolicy {
    #[default]
    SameModel,
    AdvanceOnConfirmedNonacceptance,
}
impl FailoverPolicy {
    pub(crate) fn is_same_model(&self) -> bool {
        *self == Self::SameModel
    }
}
