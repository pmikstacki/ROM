#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid { kind: String, field: String },
    Unsupported(String),
    Duplicate(String),
    Unregistered,
    Denied,
    Conflict,
    IdentityMismatch,
    IdentityExpired,
    Missing,
    Overloaded,
    Closed,
    NotCommitted,
    Unknown,
    Panicked,
    Storage,
    TooLarge,
    HistoryGap,
}
impl Error {
    pub fn invalid(kind: &str, field: &str) -> Self {
        Self::Invalid {
            kind: kind.into(),
            field: field.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
