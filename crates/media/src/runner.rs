//! Bus integration: react to media control commands and announce media state.
//!
//! This module connects the media service to the bus. It subscribes, watches for
//! [`MediaControl`](dash_core::EventKind::MediaControl) commands (produced by the
//! voice service or the gateway), runs them through the
//! [`v1::MediaApi`](crate::v1::MediaApi), and publishes the resulting
//! [`MediaState`](dash_core::EventKind::MediaState) back onto the bus.
//!
//! Natural-language parsing lives in the voice service, not here — media only
//! ever sees an already-structured [`MediaAction`].

use crate::api::v1::MediaApi;
use crate::service::MediaService;
use dash_bus::{Bus, Subscription};
use dash_core::{CoreError, Event, EventKind, MediaAction, ServiceId};
use std::sync::Arc;

/// Subscribe to `bus` and spawn the media loop, returning its join handle.
///
/// Creates the [`Subscription`] **synchronously** (so no event published after
/// this call can be missed) and then spawns [`run`] onto the tokio runtime.
pub fn spawn(service: Arc<MediaService>, bus: Bus) -> tokio::task::JoinHandle<Result<(), CoreError>> {
    let sub = bus.subscribe();
    tokio::spawn(run(service, bus, sub))
}

/// Run the media service's bus loop until the bus is closed.
///
/// For every [`MediaControl`](EventKind::MediaControl) on `sub`, executes the
/// action against `service` and publishes the resulting
/// [`MediaState`](EventKind::MediaState) on `bus`. Service errors (e.g. an empty
/// playlist) are logged to stderr and skipped — one bad command must not take the
/// loop down.
///
/// Returns `Ok(())` when the bus closes (all senders dropped).
pub async fn run(
    service: Arc<MediaService>,
    bus: Bus,
    mut sub: Subscription,
) -> Result<(), CoreError> {
    loop {
        let event = match sub.recv().await {
            Ok(ev) => ev,
            Err(CoreError::BusClosed) => return Ok(()),
            Err(CoreError::Lagged(n)) => {
                eprintln!("[media] lagged, skipped {n} event(s)");
                continue;
            }
            Err(e) => return Err(e),
        };

        // Only structured media control commands drive the service.
        let action = match event.kind {
            EventKind::MediaControl { action } => action,
            _ => continue,
        };

        let result = match action {
            MediaAction::Play => service.play().await,
            MediaAction::Pause => service.pause().await,
            MediaAction::Next => service.next_track().await,
        };

        match result {
            Ok(state) => {
                bus.publish(Event::new(
                    ServiceId::Media,
                    EventKind::MediaState {
                        playing: state.playing,
                        track: state.current_title(),
                    },
                ));
            }
            Err(e) => eprintln!("[media] action {action:?} failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    fn control(action: MediaAction) -> Event {
        Event::new(ServiceId::Voice, EventKind::MediaControl { action })
    }

    #[tokio::test]
    async fn media_control_play_produces_media_state_on_bus() {
        let bus = Bus::new();
        let service = Arc::new(MediaService::with_demo_tracks());

        // Probe subscribes BEFORE the command is published so it sees the result.
        let mut probe = bus.subscribe();

        // `spawn` subscribes synchronously, so the command below can't be missed.
        let handle = spawn(service, bus.clone());

        bus.publish(control(MediaAction::Play));

        let state = timeout(Duration::from_secs(1), async {
            loop {
                let ev = probe.recv().await.unwrap();
                if let EventKind::MediaState { playing, track } = ev.kind {
                    assert_eq!(ev.source, ServiceId::Media);
                    return (playing, track);
                }
            }
        })
        .await
        .expect("timed out waiting for MediaState");

        assert_eq!(state, (true, Some("Highway Star".to_string())));
        handle.abort();
    }

    #[tokio::test]
    async fn non_media_events_are_ignored() {
        let bus = Bus::new();
        let service = Arc::new(MediaService::with_demo_tracks());
        let mut probe = bus.subscribe();
        let handle = spawn(service, bus.clone());

        // A voice command is not a media control command; media must ignore it.
        bus.publish(Event::new(
            ServiceId::Voice,
            EventKind::VoiceCommand { transcript: "play music".into() },
        ));
        // Then a real control command; media should react only to this one.
        bus.publish(control(MediaAction::Pause));

        // The only MediaState the media service emits should be from the Pause.
        let playing = timeout(Duration::from_secs(1), async {
            loop {
                let ev = probe.recv().await.unwrap();
                if let EventKind::MediaState { playing, .. } = ev.kind {
                    return playing;
                }
            }
        })
        .await
        .expect("timed out waiting for MediaState");
        assert!(!playing);
        handle.abort();
    }
}
