//! Error type for the `litert-lm` crate.

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the safe LiteRT-LM API.
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("engine creation failed (returned null)")]
    EngineCreationFailed,

    #[error("session creation failed (returned null)")]
    SessionCreationFailed,

    #[error("generation failed: {0}")]
    GenerationFailed(String),

    #[error("null pointer returned from the LiteRT-LM C API")]
    NullPointer,

    #[error("path contains non-UTF-8 characters: {0:?}")]
    InvalidPath(std::path::PathBuf),
}
