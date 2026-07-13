//! Errors returned by the settings service.

use dash_core::CoreError;
use thiserror::Error;

/// Failures from the settings service.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceError {
    /// The setting key is not one this service knows about.
    #[error("unknown setting key: {0:?}")]
    UnknownKey(String),

    /// The value is not valid for the given key.
    #[error("invalid value {value:?} for setting {key:?}")]
    InvalidValue {
        /// The setting key.
        key: String,
        /// The rejected value.
        value: String,
    },

    /// A failure in shared infrastructure (bus transport, serialization).
    #[error(transparent)]
    Core(#[from] CoreError),
}
