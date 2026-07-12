//! # dash-core
//!
//! Foundational types shared across every crate in the `dash` workspace.
//!
//! This crate deliberately contains **no business logic and no async runtime
//! dependencies**. It only defines the vocabulary that services, the bus, and
//! the gateway all agree on:
//!
//! - [`Event`] — the envelope that travels across the message bus.
//! - [`ServiceId`] — which service produced (or should handle) a message.
//! - [`CoreError`] — low-level failures shared by the whole system.
//!
//! Keeping these types in a dependency-light crate means a service can depend on
//! the *vocabulary* without being forced to depend on the *transport* (the bus)
//! or on every other service.

pub mod error;
pub mod event;
pub mod ids;

pub use error::CoreError;
pub use event::{Event, EventKind};
pub use ids::ServiceId;
