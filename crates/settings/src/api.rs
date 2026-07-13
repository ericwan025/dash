//! The versioned public API of the settings service.

/// Version 1 of the settings service API.
pub mod v1 {
    use crate::error::ServiceError;
    use crate::types::SettingsSnapshot;
    use async_trait::async_trait;

    /// The settings contract.
    ///
    /// Same conventions as the other services: `async`, fallible, `&self`.
    #[async_trait]
    pub trait SettingsApi: Send + Sync {
        /// Set `key` to `value`, returning the normalized value actually stored.
        ///
        /// # Errors
        /// [`ServiceError::UnknownKey`] or [`ServiceError::InvalidValue`] if the
        /// key or value is not accepted.
        async fn set(&self, key: &str, value: &str) -> Result<String, ServiceError>;

        /// Read a single setting's value, if the key exists.
        async fn get(&self, key: &str) -> Result<Option<String>, ServiceError>;

        /// Read a snapshot of all settings.
        async fn snapshot(&self) -> Result<SettingsSnapshot, ServiceError>;
    }
}
