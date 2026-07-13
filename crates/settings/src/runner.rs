//! Bus integration: react to setting-change commands and announce new state.
//!
//! Subscribes to the bus and, for every
//! [`SetSetting`](dash_core::EventKind::SetSetting) command, applies it via the
//! [`SettingsService`] and publishes the resulting
//! [`SettingsState`](dash_core::EventKind::SettingsState). Invalid commands are
//! logged and dropped.

use crate::api::v1::SettingsApi;
use crate::service::SettingsService;
use dash_bus::{Bus, Subscription};
use dash_core::{CoreError, Event, EventKind, ServiceId};
use std::sync::Arc;

/// Subscribe to `bus` and spawn the settings loop, returning its join handle.
pub fn spawn(
    service: Arc<SettingsService>,
    bus: Bus,
) -> tokio::task::JoinHandle<Result<(), CoreError>> {
    let sub = bus.subscribe();
    tokio::spawn(run(service, bus, sub))
}

/// Run the settings service's bus loop until the bus is closed.
pub async fn run(
    service: Arc<SettingsService>,
    bus: Bus,
    mut sub: Subscription,
) -> Result<(), CoreError> {
    loop {
        let event = match sub.recv().await {
            Ok(ev) => ev,
            Err(CoreError::BusClosed) => return Ok(()),
            Err(CoreError::Lagged(n)) => {
                eprintln!("[settings] lagged, skipped {n} event(s)");
                continue;
            }
            Err(e) => return Err(e),
        };

        let (key, value) = match event.kind {
            EventKind::SetSetting { key, value } => (key, value),
            _ => continue,
        };

        match service.set(&key, &value).await {
            Ok(normalized) => {
                bus.publish(Event::new(
                    ServiceId::Settings,
                    EventKind::SettingsState {
                        key,
                        value: normalized,
                    },
                ));
            }
            Err(e) => eprintln!("[settings] set failed: {e}"),
        }
    }
}
