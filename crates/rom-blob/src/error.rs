//! Blob boundary failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    TooLarge,
    Input,
    Timeout,
    Conflict,
    Missing,
    Denied,
    Unsupported,
    Backend,
    Unknown,
    Overloaded,
    Closed,
    Panicked,
    Core(rom::Error),
}
impl From<rom::Error> for Error {
    fn from(error: rom::Error) -> Self {
        Self::Core(error)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
