//! Domain types for the media service.

use serde::{Deserialize, Serialize};

/// A single playable track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    /// Track title, e.g. `"Highway Star"`.
    pub title: String,
    /// Performing artist, e.g. `"Deep Purple"`.
    pub artist: String,
}

impl Track {
    /// Convenience constructor.
    pub fn new(title: impl Into<String>, artist: impl Into<String>) -> Self {
        Track {
            title: title.into(),
            artist: artist.into(),
        }
    }
}

/// A snapshot of the media service's playback state.
///
/// Returned by every [`v1::MediaApi`](crate::v1::MediaApi) method so callers
/// always see the resulting state, and mirrored onto the bus as
/// [`EventKind::MediaState`](dash_core::EventKind).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackState {
    /// Whether audio is currently playing.
    pub playing: bool,
    /// The currently selected track, if the playlist is non-empty.
    pub current: Option<Track>,
}

impl PlaybackState {
    /// The title of the current track, if any — handy for the bus event.
    pub fn current_title(&self) -> Option<String> {
        self.current.as_ref().map(|t| t.title.clone())
    }
}
