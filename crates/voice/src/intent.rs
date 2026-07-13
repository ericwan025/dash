//! The [`Intent`] type: a structured interpretation of a voice transcript.

use dash_core::{Event, EventKind, MediaAction, ServiceId};

/// A recognized user intent, produced by parsing a transcript.
///
/// Each intent corresponds to exactly one command [`EventKind`] that the voice
/// service publishes for the owning service to consume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Control media playback.
    Media(MediaAction),
    /// Navigate to a destination.
    Navigate {
        /// Where to go.
        destination: String,
    },
    /// Change a setting.
    Setting {
        /// Setting key, e.g. `"volume"`.
        key: String,
        /// New value.
        value: String,
    },
}

impl Intent {
    /// Convert this intent into the bus command [`Event`] the voice service
    /// publishes (sourced from [`ServiceId::Voice`]).
    pub fn into_event(self) -> Event {
        let kind = match self {
            Intent::Media(action) => EventKind::MediaControl { action },
            Intent::Navigate { destination } => EventKind::SetDestination { destination },
            Intent::Setting { key, value } => EventKind::SetSetting { key, value },
        };
        Event::new(ServiceId::Voice, kind)
    }
}
