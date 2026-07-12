//! The versioned public API of the media service.
//!
//! ## Why versioned?
//!
//! The service's trait is the contract every other part of the system codes
//! against. Nesting it in a `v1` module means we can introduce a `v2` trait
//! later — with new or changed methods — while the old `v1` implementation keeps
//! working for existing callers. Consumers pick a version explicitly:
//!
//! ```ignore
//! use dash_media::v1::MediaApi;
//! ```
//!
//! This is the same shape every service in `dash` follows (`nav::v1::NavApi`,
//! `settings::v1::SettingsApi`, …), so the pattern is worth getting right here
//! first.

/// Version 1 of the media service API.
pub mod v1 {
    use crate::error::ServiceError;
    use crate::types::PlaybackState;
    use async_trait::async_trait;

    /// The media playback contract.
    ///
    /// All methods are `async` and fallible, returning
    /// [`Result<PlaybackState, ServiceError>`]. Every successful call returns the
    /// **resulting** [`PlaybackState`] so callers never need a separate "read"
    /// round-trip after an action.
    ///
    /// Methods take `&self` (not `&mut self`) so an implementation can be shared
    /// behind an `Arc` across the async runtime and use interior mutability for
    /// its state.
    #[async_trait]
    pub trait MediaApi: Send + Sync {
        /// Start (or resume) playback of the current track.
        ///
        /// # Errors
        /// [`ServiceError::EmptyPlaylist`] if no tracks are loaded.
        async fn play(&self) -> Result<PlaybackState, ServiceError>;

        /// Pause playback. Pausing an already-paused player is a no-op and
        /// succeeds.
        async fn pause(&self) -> Result<PlaybackState, ServiceError>;

        /// Advance to the next track, wrapping around to the first after the
        /// last. Does not change whether audio is playing.
        ///
        /// # Errors
        /// [`ServiceError::EmptyPlaylist`] if no tracks are loaded.
        async fn next_track(&self) -> Result<PlaybackState, ServiceError>;

        /// Select the track at `index` (0-based) without changing play/pause.
        ///
        /// # Errors
        /// [`ServiceError::TrackOutOfRange`] if `index` is not a valid track.
        async fn select_track(&self, index: usize) -> Result<PlaybackState, ServiceError>;

        /// Read the current playback state without changing anything.
        async fn now_playing(&self) -> Result<PlaybackState, ServiceError>;
    }
}
