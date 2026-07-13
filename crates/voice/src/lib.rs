//! # dash-voice
//!
//! The voice service: the system's natural-language front door.
//!
//! It is **stateless**. Its whole job is to turn a raw transcript into a
//! structured [`Intent`] ([`v1::VoiceApi::parse`]) and, on the bus, to republish
//! that intent as the command event the owning service consumes:
//!
//! ```text
//! VoiceCommand{"play music"}  --voice-->  MediaControl{Play}   --> media
//! VoiceCommand{"go home"}     --voice-->  SetDestination{Home} --> nav
//! VoiceCommand{"set volume to 7"} --voice--> SetSetting{...}   --> settings
//! ```
//!
//! Follows the same shape as [`dash_media`](../dash_media): [`error`], [`api`]
//! (`v1`), [`service`], [`runner`].

pub mod api;
pub mod error;
pub mod intent;
pub mod runner;
pub mod service;

pub use api::v1;
pub use error::ServiceError;
pub use intent::Intent;
pub use runner::{run, spawn};
pub use service::{parse_intent, VoiceService};
