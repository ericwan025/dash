//! Bus integration: turn raw voice transcripts into structured commands.
//!
//! The voice runner subscribes to the bus, and for every
//! [`VoiceCommand`](dash_core::EventKind::VoiceCommand) it parses the transcript
//! and, on success, publishes the corresponding command event (e.g.
//! [`MediaControl`](dash_core::EventKind::MediaControl)) for the owning service
//! to consume. Unrecognized transcripts are logged and dropped.

use crate::api::v1::VoiceApi;
use crate::service::VoiceService;
use dash_bus::{Bus, Subscription};
use dash_core::{CoreError, EventKind};
use std::sync::Arc;

/// Subscribe to `bus` and spawn the voice loop, returning its join handle.
pub fn spawn(service: Arc<VoiceService>, bus: Bus) -> tokio::task::JoinHandle<Result<(), CoreError>> {
    let sub = bus.subscribe();
    tokio::spawn(run(service, bus, sub))
}

/// Run the voice service's bus loop until the bus is closed.
pub async fn run(
    service: Arc<VoiceService>,
    bus: Bus,
    mut sub: Subscription,
) -> Result<(), CoreError> {
    loop {
        let event = match sub.recv().await {
            Ok(ev) => ev,
            Err(CoreError::BusClosed) => return Ok(()),
            Err(CoreError::Lagged(n)) => {
                eprintln!("[voice] lagged, skipped {n} event(s)");
                continue;
            }
            Err(e) => return Err(e),
        };

        let transcript = match event.kind {
            EventKind::VoiceCommand { transcript } => transcript,
            _ => continue,
        };

        match service.parse(&transcript).await {
            Ok(intent) => {
                bus.publish(intent.into_event());
            }
            Err(e) => eprintln!("[voice] {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_core::{Event, MediaAction, ServiceId};
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn transcript_becomes_media_control_on_bus() {
        let bus = Bus::new();
        let mut probe = bus.subscribe();
        let handle = spawn(Arc::new(VoiceService::new()), bus.clone());

        // A raw utterance enters the bus (as if from the gateway).
        bus.publish(Event::new(
            ServiceId::Gateway,
            EventKind::VoiceCommand { transcript: "play music".into() },
        ));

        // Voice should republish it as a structured MediaControl command.
        let action = timeout(Duration::from_secs(1), async {
            loop {
                let ev = probe.recv().await.unwrap();
                if let EventKind::MediaControl { action } = ev.kind {
                    assert_eq!(ev.source, ServiceId::Voice);
                    return action;
                }
            }
        })
        .await
        .expect("timed out waiting for MediaControl");

        assert_eq!(action, MediaAction::Play);
        handle.abort();
    }

    #[tokio::test]
    async fn unrecognized_transcript_produces_no_command() {
        let bus = Bus::new();
        let mut probe = bus.subscribe();
        let handle = spawn(Arc::new(VoiceService::new()), bus.clone());

        bus.publish(Event::new(
            ServiceId::Gateway,
            EventKind::VoiceCommand { transcript: "hello there".into() },
        ));

        // No command should be published; only our own VoiceCommand echoes back.
        let first = timeout(Duration::from_millis(300), probe.recv()).await;
        if let Ok(Ok(ev)) = first {
            assert!(
                matches!(ev.kind, EventKind::VoiceCommand { .. }),
                "unexpected command from unrecognized transcript: {:?}",
                ev.kind
            );
        }
        handle.abort();
    }
}
