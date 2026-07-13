//! Errors returned by the navigation service.

use dash_core::CoreError;
use thiserror::Error;

/// Failures from the navigation service.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceError {
    /// A destination was requested but the text was empty/blank.
    #[error("destination must not be empty")]
    EmptyDestination,

    /// A failure in shared infrastructure (bus transport, serialization).
    #[error(transparent)]
    Core(#[from] CoreError),
}
