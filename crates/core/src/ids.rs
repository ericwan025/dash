//! Identifiers for the services that make up the system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies a single service in the `dash` system.
///
/// Every [`Event`](crate::Event) records the [`ServiceId`] that produced it, so
/// subscribers can filter by source and the gateway can label state updates it
/// forwards to the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceId {
    /// Turn-by-turn navigation and destination management.
    Nav,
    /// Audio playback (play / pause / track state).
    Media,
    /// Speech-to-intent parsing; the source of most commands.
    Voice,
    /// User-facing settings (volume, theme, units).
    Settings,
    /// The WebSocket bridge to the Flutter frontend.
    Gateway,
}

impl ServiceId {
    /// A stable, lowercase string name — handy for logs and JSON keys.
    pub const fn as_str(self) -> &'static str {
        match self {
            ServiceId::Nav => "nav",
            ServiceId::Media => "media",
            ServiceId::Voice => "voice",
            ServiceId::Settings => "settings",
            ServiceId::Gateway => "gateway",
        }
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
