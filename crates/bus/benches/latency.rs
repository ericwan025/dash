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
//! `harness = false` (see Cargo.toml) means this file owns `main`.
//!
//! ## Method
//!
//! A subscriber runs on its own task. For each sample the publisher records
//! `t0`, calls `publish`, and the subscriber stamps `t_recv` the instant its
//! `recv().await` returns and sends that instant back over an mpsc channel. The
//! reported latency is `t_recv - t0` — capturing broadcast delivery plus the
//! task wake-up, and nothing else (the mpsc hop happens *after* `t_recv` is
//! taken, so it never inflates the measurement).

use dash_bus::Bus;
use dash_core::{Event, EventKind, ServiceId};
use std::time::{Duration, Instant};

/// Number of measured samples.
const SAMPLES: usize = 20_000;
/// Warm-up iterations discarded before measuring (let the runtime settle).
const WARMUP: usize = 1_000;

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
    // Generous capacity so the subscriber never lags during the run.
    let bus = Bus::with_capacity(SAMPLES + WARMUP + 16);
    let mut sub = bus.subscribe();

    // Subscriber task: stamp the receive instant and hand it back.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Instant>();
    tokio::spawn(async move {
        while sub.recv().await.is_ok() {
            let t_recv = Instant::now();
            if tx.send(t_recv).is_err() {
                break;
            }
        }
    });

    // One publish -> one receipt, measured per sample.
    async fn round(bus: &Bus, rx: &mut tokio::sync::mpsc::UnboundedReceiver<Instant>) -> Duration {
        let t0 = Instant::now();
        bus.publish(sample_event());
        let t_recv = rx.recv().await.expect("subscriber closed");
        t_recv - t0
    }

    for _ in 0..WARMUP {
        round(&bus, &mut rx).await;
    }

    let mut samples: Vec<Duration> = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        samples.push(round(&bus, &mut rx).await);
    }

    let total: Duration = samples.iter().sum();
    let avg = total / samples.len() as u32;

    println!("dash-bus publish -> subscriber latency");
    println!("  samples: {SAMPLES}");
    println!("  average: {avg:?}");
}
