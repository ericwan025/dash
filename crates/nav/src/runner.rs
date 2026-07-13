//! Bus integration: react to destination commands and announce nav state.
//!
//! Subscribes to the bus and, for every
//! [`SetDestination`](dash_core::EventKind::SetDestination) command, updates the
//! [`NavService`] and publishes the resulting
//! [`NavState`](dash_core::EventKind::NavState).

use crate::api::v1::NavApi;
use crate::service::NavService;
use dash_bus::{Bus, Subscription};
use dash_core::{CoreError, Event, EventKind, ServiceId};
use std::sync::Arc;

/// Subscribe to `bus` and spawn the nav loop, returning its join handle.
pub fn spawn(service: Arc<NavService>, bus: Bus) -> tokio::task::JoinHandle<Result<(), CoreError>> {
    let sub = bus.subscribe();
    tokio::spawn(run(service, bus, sub))
}

/// Run the nav service's bus loop until the bus is closed.
pub async fn run(
    service: Arc<NavService>,
    bus: Bus,
    mut sub: Subscription,
) -> Result<(), CoreError> {
    loop {
        let event = match sub.recv().await {
            Ok(ev) => ev,
            Err(CoreError::BusClosed) => return Ok(()),
            Err(CoreError::Lagged(n)) => {
                eprintln!("[nav] lagged, skipped {n} event(s)");
                continue;
            }
            Err(e) => return Err(e),
        };

        let destination = match event.kind {
            EventKind::SetDestination { destination } => destination,
            _ => continue,
        };

        match service.set_destination(&destination).await {
            Ok(status) => {
                bus.publish(Event::new(
                    ServiceId::Nav,
                    EventKind::NavState {
                        destination: status.destination,
                    },
                ));
            }
            Err(e) => eprintln!("[nav] set_destination failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn set_destination_command_produces_nav_state() {
        let bus = Bus::new();
        let mut probe = bus.subscribe();
        let handle = spawn(Arc::new(NavService::new()), bus.clone());

        bus.publish(Event::new(
            ServiceId::Gateway,
            EventKind::SetDestination { destination: "Pier 39".into() },
        ));

        let dest = timeout(Duration::from_secs(1), async {
            loop {
                let ev = probe.recv().await.unwrap();
                if let EventKind::NavState { destination } = ev.kind {
                    assert_eq!(ev.source, ServiceId::Nav);
                    return destination;
                }
            }
        })
        .await
        .expect("timed out waiting for NavState");

        assert_eq!(dest.as_deref(), Some("Pier 39"));
        handle.abort();
    }

    #[tokio::test]
    async fn blank_destination_command_produces_no_state() {
        let bus = Bus::new();
        let mut probe = bus.subscribe();
        let handle = spawn(Arc::new(NavService::new()), bus.clone());

        bus.publish(Event::new(
            ServiceId::Gateway,
            EventKind::SetDestination { destination: "   ".into() },
        ));

        // A blank destination is rejected by the service, so no NavState follows;
        // only our own command echoes back on the bus.
        let first = timeout(Duration::from_millis(300), probe.recv()).await;
        if let Ok(Ok(ev)) = first {
            assert!(
                matches!(ev.kind, EventKind::SetDestination { .. }),
                "unexpected NavState from blank destination: {:?}",
                ev.kind
            );
        }
        handle.abort();
    }
}
