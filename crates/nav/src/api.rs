//! The versioned public API of the navigation service.

/// Version 1 of the navigation service API.
pub mod v1 {
    use crate::error::ServiceError;
    use crate::types::NavStatus;
    use async_trait::async_trait;

    /// The navigation contract.
    ///
    /// Follows the same conventions as the other services: `async`, fallible,
    /// `&self` (so it can be shared behind an `Arc`), and every successful call
    /// returns the resulting [`NavStatus`].
    #[async_trait]
    pub trait NavApi: Send + Sync {
        /// Set the active destination.
        ///
        /// # Errors
        /// [`ServiceError::EmptyDestination`] if `destination` is blank.
        async fn set_destination(&self, destination: &str) -> Result<NavStatus, ServiceError>;

        /// Clear the destination, returning navigation to idle. Always succeeds.
        async fn clear(&self) -> Result<NavStatus, ServiceError>;

        /// Read the current navigation status without changing anything.
        async fn current(&self) -> Result<NavStatus, ServiceError>;
    }
}
