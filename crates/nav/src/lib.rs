//! # dash-nav
//!
//! The navigation service. Owns the active destination and announces changes as
//! [`NavState`](dash_core::EventKind::NavState) on the bus.
//!
//! Same shape as the other services: [`error`], [`types`], [`api`] (`v1`),
//! [`service`]. The bus [`runner`] is added next.

pub mod api;
pub mod error;
pub mod runner;
pub mod service;
pub mod types;

pub use api::v1;
pub use error::ServiceError;
pub use runner::{run, spawn};
pub use service::NavService;
pub use types::NavStatus;
