//! Low-level errors shared across the whole system.
//!
//! Each service defines its *own* rich error enum (for example
//! `media::ServiceError`) for domain-specific failures. [`CoreError`] only
//! covers failures in the shared plumbing — serialization and bus transport —
//! so that every service's error type can wrap it uniformly.

use thiserror::Error;

/// Errors originating in shared infrastructure rather than a specific service's
/// domain logic.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    /// An [`Event`](crate::Event) payload could not be (de)serialized to/from
    /// JSON. Carries the underlying `serde_json` message.
    #[error("failed to (de)serialize event payload: {0}")]
    Serialization(String),

    /// A subscriber tried to receive from the bus but the channel was closed,
    /// or a publisher tried to send with no receivers alive.
    #[error("message bus channel closed")]
    BusClosed,

    /// A subscriber fell behind and the broadcast channel dropped messages it
    /// never read. Carries the number of skipped messages.
    #[error("subscriber lagged and missed {0} message(s)")]
    Lagged(u64),
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        CoreError::Serialization(e.to_string())
    }
}
