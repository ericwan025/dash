//! The [`Event`] envelope and its [`EventKind`] payload.
//!
//! An [`Event`] is the single unit that travels across the message bus. It pairs
//! a small amount of **metadata** (who sent it, when, and a unique id) with a
//! **payload** ([`EventKind`]).
//!
//! [`EventKind`] is a `serde`-tagged enum, so it serializes to compact,
//! self-describing JSON — which is exactly what the gateway forwards to the
//! Flutter frontend:
//!
//! ```json
//! { "type": "media_state", "playing": true, "track": "Highway Star" }
//! ```
//!
//! Payloads fall into two conceptual groups:
//!
//! - **Commands** — a request for a service to *do* something (e.g.
//!   [`EventKind::VoiceCommand`]). These usually originate from voice or the
//!   frontend.
//! - **State updates** — a service announcing a *fact* about its new state
//!   (e.g. [`EventKind::MediaState`]). The gateway relays these to the UI.

use crate::ids::ServiceId;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// A message travelling across the bus: metadata plus a typed [`EventKind`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Unique id for this event, useful for tracing and de-duplication.
    pub id: Uuid,
    /// The service that produced the event.
    pub source: ServiceId,
    /// Milliseconds since the Unix epoch, captured when the event was created.
    pub ts_millis: u128,
    /// The typed payload.
    pub kind: EventKind,
}

impl Event {
    /// Create a new event from `source` carrying `kind`, stamping it with a
    /// fresh id and the current time.
    pub fn new(source: ServiceId, kind: EventKind) -> Self {
        Event {
            id: Uuid::new_v4(),
            source,
            ts_millis: now_millis(),
            kind,
        }
    }
}

/// A media transport action, carried by [`EventKind::MediaControl`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaAction {
    /// Start or resume playback.
    Play,
    /// Pause playback.
    Pause,
    /// Skip to the next track.
    Next,
}

/// The payload of an [`Event`].
///
/// Variants split into two groups, and this split is the backbone of the whole
/// system:
///
/// - **Commands** — a *request* for a service to act. Produced by the frontend
///   (via the gateway) or by the voice service, and consumed by exactly one
///   owning service: [`VoiceCommand`](EventKind::VoiceCommand) → voice,
///   [`MediaControl`](EventKind::MediaControl) → media,
///   [`SetDestination`](EventKind::SetDestination) → nav,
///   [`SetSetting`](EventKind::SetSetting) → settings.
/// - **State** — a *fact* a service announces after acting:
///   [`MediaState`](EventKind::MediaState), [`NavState`](EventKind::NavState),
///   [`SettingsState`](EventKind::SettingsState). The gateway relays these to the
///   UI.
///
/// The `#[serde(tag = "type")]` attribute means each variant serializes with a
/// `"type"` discriminant field, giving the frontend a single field to switch on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    // ----- Commands -----
    /// A raw voice utterance for the voice service to interpret, e.g.
    /// `"play music"`. The voice service turns this into a more specific command
    /// (like [`MediaControl`](EventKind::MediaControl)).
    VoiceCommand {
        /// The raw recognized text.
        transcript: String,
    },

    /// A structured request to control media playback. Consumed by media.
    MediaControl {
        /// What to do.
        action: MediaAction,
    },

    /// A request to set the navigation destination. Consumed by nav.
    SetDestination {
        /// Human-readable destination, e.g. `"1600 Amphitheatre Pkwy"`.
        destination: String,
    },

    /// A request to change a user setting. Consumed by settings.
    SetSetting {
        /// Setting key, e.g. `"volume"` or `"theme"`.
        key: String,
        /// New value, serialized as a string for schema simplicity.
        value: String,
    },

    // ----- State -----
    /// Media playback state changed. Produced by the media service.
    MediaState {
        /// Whether audio is currently playing.
        playing: bool,
        /// The current track title, if any.
        track: Option<String>,
    },

    /// Navigation state changed. Produced by the nav service.
    NavState {
        /// The active destination, or `None` if navigation is idle.
        destination: Option<String>,
    },

    /// A user setting's value changed. Produced by the settings service.
    SettingsState {
        /// Setting key.
        key: String,
        /// New value.
        value: String,
    },
}

/// Current time in milliseconds since the Unix epoch.
///
/// If the system clock is before the epoch (should never happen on real
/// hardware) this saturates to `0` rather than panicking.
fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_state_serializes_with_type_tag() {
        // The frontend switches on the `type` field, so pin the exact shape.
        let kind = EventKind::MediaState {
            playing: true,
            track: Some("Highway Star".to_string()),
        };
        let json = serde_json::to_value(&kind).unwrap();
        assert_eq!(json["type"], "media_state");
        assert_eq!(json["playing"], true);
        assert_eq!(json["track"], "Highway Star");
    }

    #[test]
    fn voice_command_round_trips() {
        let kind = EventKind::VoiceCommand {
            transcript: "play music".to_string(),
        };
        let json = serde_json::to_string(&kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn event_envelope_round_trips_over_json() {
        let event = Event::new(
            ServiceId::Media,
            EventKind::MediaState {
                playing: false,
                track: None,
            },
        );
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
        assert_eq!(back.source, ServiceId::Media);
    }

    #[test]
    fn null_track_omitted_value_deserializes() {
        // A media_state with an explicit null track should parse to None.
        let json = r#"{ "type": "media_state", "playing": false, "track": null }"#;
        let kind: EventKind = serde_json::from_str(json).unwrap();
        assert_eq!(
            kind,
            EventKind::MediaState {
                playing: false,
                track: None
            }
        );
    }
}
