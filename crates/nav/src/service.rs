//! The concrete navigation service: destination state + the [`v1::NavApi`] impl.

use crate::api::v1::NavApi;
use crate::error::ServiceError;
use crate::types::NavStatus;
use async_trait::async_trait;
use std::sync::Mutex;

/// The navigation service. Holds the active destination behind a [`Mutex`].
///
/// Like [`MediaService`](../dash_media), the lock is only held for short,
/// synchronous updates — never across an `.await` — so a `std::sync::Mutex` is
/// correct and cheap.
#[derive(Debug, Default)]
pub struct NavService {
    destination: Mutex<Option<String>>,
}

impl NavService {
    /// Create an idle navigation service with no destination.
    pub fn new() -> Self {
        NavService::default()
    }

    fn snapshot(dest: &Option<String>) -> NavStatus {
        NavStatus {
            destination: dest.clone(),
        }
    }
}

#[async_trait]
impl NavApi for NavService {
    async fn set_destination(&self, destination: &str) -> Result<NavStatus, ServiceError> {
        let trimmed = destination.trim();
        if trimmed.is_empty() {
            return Err(ServiceError::EmptyDestination);
        }
        let mut dest = self.destination.lock().expect("nav state lock poisoned");
        *dest = Some(trimmed.to_string());
        Ok(Self::snapshot(&dest))
    }

    async fn clear(&self) -> Result<NavStatus, ServiceError> {
        let mut dest = self.destination.lock().expect("nav state lock poisoned");
        *dest = None;
        Ok(Self::snapshot(&dest))
    }

    async fn current(&self) -> Result<NavStatus, ServiceError> {
        let dest = self.destination.lock().expect("nav state lock poisoned");
        Ok(Self::snapshot(&dest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_destination_updates_state() {
        let nav = NavService::new();
        let status = nav.set_destination("Pier 39").await.unwrap();
        assert_eq!(status.destination.as_deref(), Some("Pier 39"));
        // And it persists for a subsequent read.
        assert_eq!(nav.current().await.unwrap().destination.as_deref(), Some("Pier 39"));
    }

    #[tokio::test]
    async fn set_destination_trims_whitespace() {
        let nav = NavService::new();
        let status = nav.set_destination("  Home  ").await.unwrap();
        assert_eq!(status.destination.as_deref(), Some("Home"));
    }

    #[tokio::test]
    async fn blank_destination_errors() {
        let nav = NavService::new();
        assert_eq!(
            nav.set_destination("   ").await.unwrap_err(),
            ServiceError::EmptyDestination
        );
    }

    #[tokio::test]
    async fn clear_returns_to_idle() {
        let nav = NavService::new();
        nav.set_destination("Airport").await.unwrap();
        let status = nav.clear().await.unwrap();
        assert!(status.destination.is_none());
    }

    #[tokio::test]
    async fn current_starts_idle() {
        let nav = NavService::new();
        assert!(nav.current().await.unwrap().destination.is_none());
    }
}
