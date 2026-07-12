//! # dash-bus
//!
//! A tiny in-process **publish/subscribe** message bus.
//!
//! Every service in `dash` holds a clone of the same [`Bus`]. A service
//! *publishes* [`Event`]s and *subscribes* to receive every event published
//! after it subscribed. This is how, for example, the media service reacts to a
//! command emitted by the voice service without the two ever calling each other
//! directly — they only share the bus.
//!
//! ## Design
//!
//! The bus is a thin, opinionated wrapper over a [`tokio::sync::broadcast`]
//! channel:
//!
//! - **Cloneable & cheap.** [`Bus`] is just an `Arc`-like handle around a
//!   `broadcast::Sender`; clone it freely and hand one to each service.
//! - **Fan-out.** Every live [`Subscription`] receives every event. There is no
//!   per-topic routing — subscribers filter by inspecting
//!   [`Event::kind`](dash_core::Event) themselves. For a system this size that
//!   is simpler and fast enough (see the latency benchmark).
//! - **Typed errors.** Receiving maps the broadcast channel's failure modes onto
//!   [`CoreError`]: a closed channel becomes [`CoreError::BusClosed`] and a
//!   lagging subscriber becomes [`CoreError::Lagged`], so callers never see the
//!   raw tokio error types.
//!
//! ## Example
//!
//! ```
//! use dash_bus::Bus;
//! use dash_core::{Event, EventKind, ServiceId};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let bus = Bus::new();
//! let mut sub = bus.subscribe();
//!
//! bus.publish(Event::new(
//!     ServiceId::Voice,
//!     EventKind::VoiceCommand { transcript: "play music".into() },
//! ));
//!
//! let received = sub.recv().await.unwrap();
//! assert_eq!(received.source, ServiceId::Voice);
//! # }
//! ```

use dash_core::{CoreError, Event};
use tokio::sync::broadcast;

/// Default capacity of the broadcast ring buffer.
///
/// If a subscriber does not call [`Subscription::recv`] often enough and more
/// than this many events pile up, its oldest unread events are dropped and the
/// next `recv` reports [`CoreError::Lagged`].
pub const DEFAULT_CAPACITY: usize = 256;

/// A cloneable handle to the shared message bus.
///
/// Cloning a [`Bus`] is cheap and every clone talks to the same underlying
/// channel, so each service can own its own clone.
#[derive(Debug, Clone)]
pub struct Bus {
    sender: broadcast::Sender<Event>,
}

impl Bus {
    /// Create a new bus with [`DEFAULT_CAPACITY`].
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new bus whose broadcast buffer holds `capacity` events.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _rx) = broadcast::channel(capacity);
        Bus { sender }
    }

    /// Publish `event` to all current subscribers.
    ///
    /// Returns the number of subscribers the event was delivered to. Publishing
    /// with **no** subscribers is not an error — it simply returns `0`. This is
    /// intentional: services should be free to emit events even when nothing is
    /// listening yet.
    pub fn publish(&self, event: Event) -> usize {
        // `send` errors only when there are zero receivers; treat that as "0
        // delivered" rather than a failure.
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribe to the bus. The returned [`Subscription`] receives every event
    /// published *after* this call.
    pub fn subscribe(&self) -> Subscription {
        Subscription {
            receiver: self.sender.subscribe(),
        }
    }

    /// Number of live subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for Bus {
    fn default() -> Self {
        Bus::new()
    }
}

/// A subscription to the [`Bus`]. Call [`recv`](Subscription::recv) in a loop to
/// consume events.
///
/// Dropping the subscription unsubscribes it.
#[derive(Debug)]
pub struct Subscription {
    receiver: broadcast::Receiver<Event>,
}

impl Subscription {
    /// Await the next event.
    ///
    /// # Errors
    ///
    /// - [`CoreError::BusClosed`] if every [`Bus`] handle has been dropped and no
    ///   more events can ever arrive.
    /// - [`CoreError::Lagged`] if this subscriber fell behind and the channel
    ///   discarded unread events. The count of skipped events is included. The
    ///   subscription is still usable — call `recv` again to resume with the
    ///   oldest event still buffered.
    pub async fn recv(&mut self) -> Result<Event, CoreError> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Closed) => Err(CoreError::BusClosed),
            Err(broadcast::error::RecvError::Lagged(n)) => Err(CoreError::Lagged(n)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_core::{Event, EventKind, ServiceId};

    fn voice(cmd: &str) -> Event {
        Event::new(
            ServiceId::Voice,
            EventKind::VoiceCommand {
                transcript: cmd.to_string(),
            },
        )
    }

    #[tokio::test]
    async fn subscriber_receives_published_event() {
        let bus = Bus::new();
        let mut sub = bus.subscribe();
        let delivered = bus.publish(voice("play music"));
        assert_eq!(delivered, 1);

        let got = sub.recv().await.unwrap();
        assert_eq!(got.source, ServiceId::Voice);
        assert!(matches!(got.kind, EventKind::VoiceCommand { .. }));
    }

    #[tokio::test]
    async fn all_subscribers_get_every_event() {
        let bus = Bus::new();
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        let delivered = bus.publish(voice("pause"));
        assert_eq!(delivered, 2);

        assert!(a.recv().await.is_ok());
        assert!(b.recv().await.is_ok());
    }

    #[tokio::test]
    async fn publishing_with_no_subscribers_is_not_an_error() {
        let bus = Bus::new();
        // No subscribers -> zero delivered, but no panic / error.
        assert_eq!(bus.publish(voice("hello")), 0);
    }

    #[tokio::test]
    async fn recv_reports_bus_closed_after_all_senders_dropped() {
        let bus = Bus::new();
        let mut sub = bus.subscribe();
        drop(bus); // last sender gone
        let err = sub.recv().await.unwrap_err();
        assert!(matches!(err, CoreError::BusClosed));
    }

    #[tokio::test]
    async fn slow_subscriber_reports_lagged() {
        // Capacity 2: publish 3 without reading -> oldest is dropped.
        let bus = Bus::with_capacity(2);
        let mut sub = bus.subscribe();
        bus.publish(voice("one"));
        bus.publish(voice("two"));
        bus.publish(voice("three"));

        match sub.recv().await {
            Err(CoreError::Lagged(n)) => assert_eq!(n, 1),
            other => panic!("expected Lagged(1), got {other:?}"),
        }
        // After a lag, the subscription still yields the surviving events.
        assert!(sub.recv().await.is_ok());
    }
}
