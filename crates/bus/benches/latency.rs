//! Message-bus latency benchmark.
//!
//! Measures the time between a publisher calling [`Bus::publish`] and a
//! subscriber on a *separate task* receiving the event — i.e. the real
//! cross-task delivery latency of the broadcast-backed bus.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p dash-bus
//! ```
//!
//! `harness = false` (see Cargo.toml) means this file owns `main`; the real
//! measurement is filled in over the next commits.

use dash_bus::Bus;
use dash_core::{Event, EventKind, ServiceId};

fn sample_event() -> Event {
    Event::new(
        ServiceId::Voice,
        EventKind::VoiceCommand {
            transcript: "play music".to_string(),
        },
    )
}

#[tokio::main]
async fn main() {
    // Sanity-check the harness wiring: one publish, one cross-task receive.
    let bus = Bus::new();
    let mut sub = bus.subscribe();
    let receiver = tokio::spawn(async move { sub.recv().await });

    bus.publish(sample_event());
    let received = receiver.await.expect("subscriber task panicked");

    assert!(received.is_ok(), "warmup event was not received");
    println!("dash-bus latency benchmark harness ready");
}
