//! Bus integration: react to voice commands and announce media state.
//!
//! This module is the media service's connection to the rest of the system. It
//! subscribes to the [`Bus`], watches for
//! [`VoiceCommand`](dash_core::EventKind::VoiceCommand) events, interprets the
//! transcript into a media action, runs it through the
//! [`v1::MediaApi`](crate::v1::MediaApi), and publishes the resulting
//! [`MediaState`](dash_core::EventKind::MediaState) back onto the bus for anyone
//! else (notably the gateway → frontend) to observe.
//!
//! Keeping this loop separate from [`MediaService`](crate::MediaService) keeps
//! the API pure: the service knows nothing about the bus, and the bus loop knows
//! nothing about the internals of playback.

use crate::api::v1::MediaApi;
use crate::service::MediaService;
use dash_bus::{Bus, Subscription};
use dash_core::{CoreError, Event, EventKind, ServiceId};
use std::sync::Arc;

/// A media action distilled from a natural-language voice transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    /// Start / resume playback.
    Play,
    /// Pause playback.
    Pause,
    /// Skip to the next track.
    Next,
}

/// Interpret a raw voice transcript into a [`MediaCommand`], if it refers to one.
///
/// This is intentionally a tiny keyword matcher — real speech-to-intent lives in
/// the voice service. It is public and pure so it can be unit-tested directly.
///
/// Returns `None` for transcripts that aren't media commands, so the runner can
/// ignore them.
pub fn interpret_command(transcript: &str) -> Option<MediaCommand> {
    let t = transcript.to_lowercase();
    // Order matters: check "pause"/"stop" and "next"/"skip" before the broad
    // "play" match so "stop playing" doesn't read as Play.
    if t.contains("pause") || t.contains("stop") {
        Some(MediaCommand::Pause)
    } else if t.contains("next") || t.contains("skip") {
        Some(MediaCommand::Next)
    } else if t.contains("play") {
        Some(MediaCommand::Play)
    } else {
        None
    }
}

/// Subscribe to `bus` and spawn the media loop, returning its join handle.
///
/// This is the convenient entry point for wiring up the service: it creates the
/// [`Subscription`] **synchronously** (so no event published after this call can
/// be missed) and then spawns [`run`] onto the tokio runtime.
pub fn spawn(service: Arc<MediaService>, bus: Bus) -> tokio::task::JoinHandle<Result<(), CoreError>> {
    let sub = bus.subscribe();
    tokio::spawn(run(service, bus, sub))
}

/// Run the media service's bus loop until the bus is closed.
///
/// For every [`VoiceCommand`] on `sub` that maps to a [`MediaCommand`], executes
/// it against `service` and publishes the resulting [`MediaState`] on `bus`.
/// Errors from the service (e.g. an empty playlist) are logged to stderr and
/// skipped — one bad command must not take the loop down.
///
/// The caller supplies `sub` so the subscription can be established before any
/// commands are published; use [`spawn`] if you don't need that control.
///
/// Returns `Ok(())` when the bus closes (all senders dropped), or an error only
/// on an unexpected non-recoverable condition.
pub async fn run(
    service: Arc<MediaService>,
    bus: Bus,
    mut sub: Subscription,
) -> Result<(), CoreError> {
    loop {
        let event = match sub.recv().await {
            Ok(ev) => ev,
            Err(CoreError::BusClosed) => return Ok(()),
            // A lagging subscriber isn't fatal; keep going with newer events.
            Err(CoreError::Lagged(n)) => {
                eprintln!("[media] lagged, skipped {n} event(s)");
                continue;
            }
            Err(e) => return Err(e),
        };

        // Only voice commands drive the media service.
        let transcript = match &event.kind {
            EventKind::VoiceCommand { transcript } => transcript,
            _ => continue,
        };

        let Some(command) = interpret_command(transcript) else {
            continue;
        };

        let result = match command {
            MediaCommand::Play => service.play().await,
            MediaCommand::Pause => service.pause().await,
            MediaCommand::Next => service.next_track().await,
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
            Err(e) => eprintln!("[media] command {command:?} failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[test]
    fn interpret_recognizes_media_commands() {
        assert_eq!(interpret_command("play music"), Some(MediaCommand::Play));
        assert_eq!(interpret_command("PLAY"), Some(MediaCommand::Play));
        assert_eq!(interpret_command("pause"), Some(MediaCommand::Pause));
        assert_eq!(interpret_command("stop the music"), Some(MediaCommand::Pause));
        assert_eq!(interpret_command("next track"), Some(MediaCommand::Next));
        assert_eq!(interpret_command("skip"), Some(MediaCommand::Next));
    }

    #[test]
    fn interpret_ignores_unrelated_transcripts() {
        assert_eq!(interpret_command("navigate home"), None);
        assert_eq!(interpret_command(""), None);
    }

    #[tokio::test]
    async fn voice_play_command_produces_media_state_on_bus() {
        let bus = Bus::new();
        let service = Arc::new(MediaService::with_demo_tracks());

        // Probe subscribes BEFORE the command is published so it sees the result.
        let mut probe = bus.subscribe();

        // Spawn the media loop; `spawn` subscribes synchronously, so the command
        // published just below cannot be missed.
        let handle = spawn(service, bus.clone());

        // Simulate the voice service emitting a parsed command.
        bus.publish(Event::new(
            ServiceId::Voice,
            EventKind::VoiceCommand {
                transcript: "play music".into(),
            },
        ));

        // The probe should eventually see a MediaState with playing = true.
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
}
