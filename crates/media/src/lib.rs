//! # dash-media
//!
//! The media playback service. This crate is the **template** every other
//! service in `dash` follows, so it is worth reading top to bottom:
//!
//! - [`error`] — the service's own [`ServiceError`] enum (`thiserror`).
//! - [`types`] — domain types ([`Track`], [`PlaybackState`]).
//! - [`v1`] — the versioned [`v1::MediaApi`] trait: the public contract.
//! - `service` (added next) — the concrete implementation plus the bus loop
//!   that reacts to voice commands.
//!
//! Nothing here talks to WebSockets or JSON directly; the service only knows
//! about the bus and its own domain. The gateway crate is responsible for
//! translating between the bus and the frontend.

pub mod api;
pub mod error;
pub mod service;
pub mod types;

pub use api::v1;
pub use error::ServiceError;
pub use service::MediaService;
pub use types::{PlaybackState, Track};
