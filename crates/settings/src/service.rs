//! The concrete settings service: a validated key/value store.

use crate::api::v1::SettingsApi;
use crate::error::ServiceError;
use crate::types::{defaults, validate, SettingsSnapshot};
use async_trait::async_trait;
use std::sync::Mutex;

/// The settings service. Holds all settings behind a [`Mutex`], starting from
/// [`defaults`].
#[derive(Debug)]
pub struct SettingsService {
    values: Mutex<SettingsSnapshot>,
}

impl SettingsService {
    /// Create a settings service pre-loaded with the default values.
    pub fn new() -> Self {
        SettingsService {
            values: Mutex::new(defaults()),
        }
    }
}

impl Default for SettingsService {
    fn default() -> Self {
        SettingsService::new()
    }
}

#[async_trait]
impl SettingsApi for SettingsService {
    async fn set(&self, key: &str, value: &str) -> Result<String, ServiceError> {
        // Validate before taking the lock; the value is normalized on success.
        let normalized = validate(key, value)?;
        let mut values = self.values.lock().expect("settings lock poisoned");
        values.insert(key.to_string(), normalized.clone());
        Ok(normalized)
    }

    async fn get(&self, key: &str) -> Result<Option<String>, ServiceError> {
        let values = self.values.lock().expect("settings lock poisoned");
        Ok(values.get(key).cloned())
    }

    async fn snapshot(&self) -> Result<SettingsSnapshot, ServiceError> {
        let values = self.values.lock().expect("settings lock poisoned");
        Ok(values.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_valid_volume_normalizes_and_stores() {
        let s = SettingsService::new();
        // Leading zero is normalized away.
        let stored = s.set("volume", "07").await.unwrap();
        assert_eq!(stored, "7");
        assert_eq!(s.get("volume").await.unwrap().as_deref(), Some("7"));
    }

    #[tokio::test]
    async fn set_valid_theme() {
        let s = SettingsService::new();
        assert_eq!(s.set("theme", "light").await.unwrap(), "light");
    }

    #[tokio::test]
    async fn volume_out_of_range_errors() {
        let s = SettingsService::new();
        assert_eq!(
            s.set("volume", "11").await.unwrap_err(),
            ServiceError::InvalidValue { key: "volume".into(), value: "11".into() }
        );
    }

    #[tokio::test]
    async fn non_numeric_volume_errors() {
        let s = SettingsService::new();
        assert!(matches!(
            s.set("volume", "loud").await.unwrap_err(),
            ServiceError::InvalidValue { .. }
        ));
    }

    #[tokio::test]
    async fn unknown_key_errors() {
        let s = SettingsService::new();
        assert_eq!(
            s.set("brightness", "5").await.unwrap_err(),
            ServiceError::UnknownKey("brightness".into())
        );
    }

    #[tokio::test]
    async fn snapshot_contains_defaults() {
        let s = SettingsService::new();
        let snap = s.snapshot().await.unwrap();
        assert_eq!(snap.get("theme").map(String::as_str), Some("dark"));
        assert_eq!(snap.get("volume").map(String::as_str), Some("5"));
        assert_eq!(snap.get("units").map(String::as_str), Some("metric"));
    }
}
