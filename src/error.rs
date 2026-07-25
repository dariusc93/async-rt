use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JoinError {
    /// The task was cancelled without an explicit abort request through its
    /// [`JoinHandle`](crate::JoinHandle), such as during runtime shutdown.
    #[error("The task was cancelled")]
    Cancelled,
    /// The task was cancelled after [`JoinHandle::abort`](crate::JoinHandle::abort)
    /// was requested.
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
