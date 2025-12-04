//! Error types for the JavaScript runtime.

/// Errors that can occur in the runtime.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Runtime has terminated")]
    Terminated,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Runtime thread panicked")]
    ThreadPanic,

    #[error("JavaScript error: {0}")]
    JavaScript(String),

    #[error("Failed to spawn thread: {0}")]
    SpawnFailed(#[from] std::io::Error),
}

/// Errors that can occur when accessing point values.
#[derive(Debug, thiserror::Error)]
pub enum PointError {
    #[error("Point not found: {0}")]
    NotFound(String),

    #[error("Write failed: {0}")]
    WriteFailed(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),
}
