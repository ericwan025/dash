//! Domain types and validation for the settings service.

use crate::error::ServiceError;
use std::collections::BTreeMap;

/// A snapshot of all settings as key → value pairs.
///
/// A `BTreeMap` so iteration order is stable (nice for tests and the UI).
pub type SettingsSnapshot = BTreeMap<String, String>;

/// The default settings the service starts with.
pub fn defaults() -> SettingsSnapshot {
    BTreeMap::from([
        ("volume".to_string(), "5".to_string()),
        ("theme".to_string(), "dark".to_string()),
        ("units".to_string(), "metric".to_string()),
    ])
}

/// Validate and normalize a `key`/`value` pair against the known settings.
///
/// Returns the normalized value to store (e.g. `"07"` → `"7"` for volume).
///
/// # Errors
/// - [`ServiceError::UnknownKey`] if `key` is not a recognized setting.
/// - [`ServiceError::InvalidValue`] if `value` is out of range / not allowed.
pub fn validate(key: &str, value: &str) -> Result<String, ServiceError> {
    match key {
        "volume" => match value.trim().parse::<u8>() {
            Ok(v) if v <= 10 => Ok(v.to_string()),
            _ => Err(ServiceError::InvalidValue {
                key: key.to_string(),
                value: value.to_string(),
            }),
        },
        "theme" => match value.trim() {
            "light" | "dark" => Ok(value.trim().to_string()),
            _ => Err(ServiceError::InvalidValue {
                key: key.to_string(),
                value: value.to_string(),
            }),
        },
        "units" => match value.trim() {
            "metric" | "imperial" => Ok(value.trim().to_string()),
            _ => Err(ServiceError::InvalidValue {
                key: key.to_string(),
                value: value.to_string(),
            }),
        },
        _ => Err(ServiceError::UnknownKey(key.to_string())),
    }
}
