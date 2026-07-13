//! # dash-settings
//!
//! The settings service: a small validated key/value store (volume, theme,
//! units). Announces changes as
//! [`SettingsState`](dash_core::EventKind::SettingsState) on the bus.
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
pub use service::SettingsService;
pub use types::{defaults, SettingsSnapshot};
