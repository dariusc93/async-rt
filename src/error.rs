use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JoinError {
    #[error("The task was cancelled")]
    Cancelled,
    #[error("The task was aborted")]
    Aborted,
    #[error("The task timed out")]
    TimedOut,
    #[error("The task panicked")]
    Panicked,
    #[error("The task was empty")]
    Empty,
    #[error("An unknown error occurred")]
    Unknown,
}

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
impl From<tokio::task::JoinError> for JoinError {
    fn from(err: tokio::task::JoinError) -> Self {
        if err.is_cancelled() {
            return JoinError::Cancelled;
        }

        if err.is_panic() {
            return JoinError::Panicked;
        }

        JoinError::Unknown
    }
}
