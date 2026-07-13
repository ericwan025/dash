//! Errors returned by the voice service.

use dash_core::CoreError;
use thiserror::Error;

/// Failures from the voice service.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceError {
    /// The transcript did not match any known command grammar.
    #[error("could not interpret transcript: {0:?}")]
    Unrecognized(String),

    /// A failure in shared infrastructure (bus transport, serialization).
    #[error(transparent)]
    Core(#[from] CoreError),
}
