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

/// The payload of an [`Event`]. See the module docs for the command/state split.
///
/// The `#[serde(tag = "type")]` attribute means each variant serializes with a
/// `"type"` discriminant field, giving the frontend a single field to switch on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    /// A parsed voice command that services may act on. Produced by the voice
    /// service (or injected by the frontend for testing).
    VoiceCommand {
        /// The raw recognized text, e.g. `"play music"`.
        transcript: String,
    },

    /// Media playback state changed. Produced by the media service.
    MediaState {
        /// Whether audio is currently playing.
        playing: bool,
        /// The current track title, if any.
        track: Option<String>,
    },

    /// A navigation destination was set. Produced by the nav service.
    NavDestination {
        /// Human-readable destination, e.g. `"1600 Amphitheatre Pkwy"`.
        destination: String,
    },

    /// A user setting changed. Produced by the settings service.
    SettingChanged {
        /// Setting key, e.g. `"volume"` or `"theme"`.
        key: String,
        /// New value, serialized as a string for schema simplicity.
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
