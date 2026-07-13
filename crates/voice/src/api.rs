//! The versioned public API of the voice service.

/// Version 1 of the voice service API.
pub mod v1 {
    use crate::error::ServiceError;
    use crate::intent::Intent;
    use async_trait::async_trait;

    /// The speech-to-intent contract.
    ///
    /// The voice service is stateless: it turns a raw transcript into a
    /// structured [`Intent`]. It does **not** act on the intent — the runner
    /// publishes it onto the bus for the owning service (media / nav / settings)
    /// to handle.
    #[async_trait]
    pub trait VoiceApi: Send + Sync {
        /// Parse a raw transcript into an [`Intent`].
        ///
        /// # Errors
        /// [`ServiceError::Unrecognized`] if the transcript matches no known
        /// command.
        async fn parse(&self, transcript: &str) -> Result<Intent, ServiceError>;
    }
}
