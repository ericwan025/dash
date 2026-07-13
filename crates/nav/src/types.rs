//! Domain types for the navigation service.

use serde::{Deserialize, Serialize};

/// A snapshot of navigation state.
///
/// Returned by every [`v1::NavApi`](crate::v1::NavApi) method and mirrored onto
/// the bus as [`EventKind::NavState`](dash_core::EventKind::NavState).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavStatus {
    /// The active destination, or `None` when navigation is idle.
    pub destination: Option<String>,
}

impl NavStatus {
    /// An idle status with no destination.
    pub fn idle() -> Self {
        NavStatus { destination: None }
    }
}
