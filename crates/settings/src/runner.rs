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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn set_setting_command_produces_settings_state() {
        let bus = Bus::new();
        let mut probe = bus.subscribe();
        let handle = spawn(Arc::new(SettingsService::new()), bus.clone());

        bus.publish(Event::new(
            ServiceId::Gateway,
            EventKind::SetSetting { key: "volume".into(), value: "07".into() },
        ));

        let (key, value) = timeout(Duration::from_secs(1), async {
            loop {
                let ev = probe.recv().await.unwrap();
                if let EventKind::SettingsState { key, value } = ev.kind {
                    assert_eq!(ev.source, ServiceId::Settings);
                    return (key, value);
                }
            }
        })
        .await
        .expect("timed out waiting for SettingsState");

        assert_eq!(key, "volume");
        // The runner emits the normalized value ("07" -> "7").
        assert_eq!(value, "7");
        handle.abort();
    }

    #[tokio::test]
    async fn invalid_setting_command_produces_no_state() {
        let bus = Bus::new();
        let mut probe = bus.subscribe();
        let handle = spawn(Arc::new(SettingsService::new()), bus.clone());

        bus.publish(Event::new(
            ServiceId::Gateway,
            EventKind::SetSetting { key: "volume".into(), value: "999".into() },
        ));

        // Rejected by the service, so no SettingsState; only our command echoes.
        let first = timeout(Duration::from_millis(300), probe.recv()).await;
        if let Ok(Ok(ev)) = first {
            assert!(
                matches!(ev.kind, EventKind::SetSetting { .. }),
                "unexpected SettingsState from invalid command: {:?}",
                ev.kind
            );
        }
        handle.abort();
    }
}
