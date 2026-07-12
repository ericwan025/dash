//! Errors returned by the media service.

use dash_core::CoreError;
use thiserror::Error;

/// Everything that can go wrong in the media service.
///
/// Every [`v1::MediaApi`](crate::v1::MediaApi) method returns
/// `Result<_, ServiceError>` — the service never panics on bad input. Domain
/// mistakes (an empty playlist, an out-of-range track) get their own variants so
/// callers can match on them, while shared plumbing failures are wrapped from
/// [`CoreError`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceError {
    /// A playback action was requested but the playlist has no tracks.
    #[error("no tracks loaded: cannot play")]
    EmptyPlaylist,

    /// A track index was requested that does not exist.
    #[error("track index {index} out of range (playlist has {len} track(s))")]
    TrackOutOfRange {
        /// The requested index.
        index: usize,
        /// The number of tracks actually loaded.
        len: usize,
    },

    /// A failure in shared infrastructure (bus transport, serialization).
    #[error(transparent)]
    Core(#[from] CoreError),
}
