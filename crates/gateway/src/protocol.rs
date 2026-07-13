//! The JSON wire protocol spoken between the gateway and the frontend.
//!
//! Two message types cross the WebSocket, both tagged with a `"type"` field so
//! the client can switch on a single key:
//!
//! - [`ClientCommand`] — **client → server**. A user action from the UI (tap
//!   play, set a destination, change a setting).
//! - [`ServerEvent`] — **server → client**. A flattened view of a bus
//!   [`Event`](dash_core::Event), so the UI can render live state.
//!
//! ## Client → server examples
//!
//! ```json
//! { "type": "voice", "transcript": "play music" }
//! { "type": "set_destination", "destination": "1600 Amphitheatre Pkwy" }
//! { "type": "set_setting", "key": "volume", "value": "7" }
//! ```
//!
//! ## Server → client example
//!
//! ```json
//! { "source": "media", "ts_millis": 1720000000000,
//!   "type": "media_state", "playing": true, "track": "Highway Star" }
//! ```

use dash_core::{Event, EventKind, ServiceId};
use serde::{Deserialize, Serialize};

/// A command sent from the frontend to the gateway.
///
/// Each variant maps onto a bus [`Event`] via [`ClientCommand::into_event`]. The
/// UI's play/pause/next buttons send [`ClientCommand::Voice`] with a short
/// transcript, reusing the exact same voice→media flow the voice service drives —
/// the buttons are just shortcuts for spoken commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    /// Inject a voice command transcript (e.g. `"play music"`, `"pause"`).
    Voice {
        /// The command text, interpreted by whichever service handles it.
        transcript: String,
    },
    /// Set the navigation destination.
    SetDestination {
        /// Human-readable destination.
        destination: String,
    },
    /// Change a user setting.
    SetSetting {
        /// Setting key, e.g. `"volume"`.
        key: String,
        /// New value as a string.
        value: String,
    },
}

impl ClientCommand {
    /// Translate this command into a bus **command** [`Event`] originating from
    /// the [`ServiceId::Gateway`].
    ///
    /// Each variant maps to the command event owned by the relevant service, so
    /// the gateway never publishes state directly — services do that in response.
    pub fn into_event(self) -> Event {
        let kind = match self {
            ClientCommand::Voice { transcript } => EventKind::VoiceCommand { transcript },
            ClientCommand::SetDestination { destination } => {
                EventKind::SetDestination { destination }
            }
            ClientCommand::SetSetting { key, value } => EventKind::SetSetting { key, value },
        };
        Event::new(ServiceId::Gateway, kind)
    }
}

/// An event forwarded from the bus to the frontend.
///
/// This is a *flattened* projection of a bus [`Event`]: the envelope's `source`
/// and `ts_millis` sit alongside the payload's own `type`-tagged fields, so the
/// client sees one flat JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerEvent {
    /// Which service produced the event.
    pub source: ServiceId,
    /// Milliseconds since the Unix epoch.
    pub ts_millis: u128,
    /// The payload, flattened so its `type` tag appears at the top level.
    #[serde(flatten)]
    pub kind: EventKind,
}

impl From<Event> for ServerEvent {
    fn from(event: Event) -> Self {
        ServerEvent {
            source: event.sourc,
            ts_millis: event.ts_millis,
            kind: event.kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_voice_command_parses() {
        let json = r#"{ "type": "voice", "transcript": "play music" }"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        assert_eq!(
            cmd,
            ClientCommand::Voice {
                transcript: "play music".into()
            }
        );
    }

    #[test]
    fn set_setting_parses() {
        let json = r#"{ "type": "set_setting", "key": "volume", "value": "7" }"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        assert_eq!(
            cmd,
            ClientCommand::SetSetting {
                key: "volume".into(),
                value: "7".into()
            }
        );
    }

    #[test]
    fn voice_command_maps_to_voice_event() {
        let ev = ClientCommand::Voice {
            transcript: "pause".into(),
        }
        .into_event();
        assert_eq!(ev.source, ServiceId::Gateway);
        assert_eq!(ev.kind, EventKind::VoiceCommand { transcript: "pause".into() });
    }

    #[test]
    fn server_event_flattens_over_json() {
        let event = Event::new(
            ServiceId::Media,
            EventKind::MediaState {
                playing: true,
                track: Some("Highway Star".into()),
            },
        );
        let json = serde_json::to_value(ServerEvent::from(event)).unwrap();
        // Envelope fields and payload fields sit at the same level.
        assert_eq!(json["source"], "media");
        assert_eq!(json["type"], "media_state");
        assert_eq!(json["playing"], true);
        assert_eq!(json["track"], "Highway Star");
    }
}
