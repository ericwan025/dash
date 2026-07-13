//! Integration tests for the gateway's bus bridge, in both directions.

use dash_bus::Bus;
use dash_core::{Event, EventKind, MediaAction, ServiceId};
use dash_media::MediaService;
use dash_nav::NavService;
use dash_settings::SettingsService;
use dash_voice::VoiceService;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Boot a gateway on an ephemeral port, returning the address and the bus so the
/// test can observe/inject events directly.
async fn spawn_gateway(bus: Bus) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, dash_gateway::app(bus)).await.unwrap();
    });
    addr
}

#[tokio::test]
async fn bus_event_reaches_client_as_json() {
    let bus = Bus::new();
    dash_media::spawn(Arc::new(MediaService::with_demo_tracks()), bus.clone());
    let addr = spawn_gateway(bus.clone()).await;

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();

    // Simulate the voice service emitting a media control command.
    bus.publish(Event::new(
        ServiceId::Voice,
        EventKind::MediaControl { action: MediaAction::Play },
    ));

    let frame = timeout(Duration::from_secs(2), async {
        loop {
            if let Message::Text(text) = ws.next().await.unwrap().unwrap() {
                let v: serde_json::Value = serde_json::from_str(&text).unwrap();
                if v["type"] == "media_state" {
                    return v;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for media_state");

    assert_eq!(frame["source"], "media");
    assert_eq!(frame["playing"], true);
    assert_eq!(frame["track"], "Highway Star");
}

#[tokio::test]
async fn client_voice_command_drives_media_end_to_end() {
    // Full pipeline: client -> gateway -> bus -> voice (NLU) -> media -> bus ->
    // gateway -> client. Exercises the real voice + media services together.
    let bus = Bus::new();
    dash_voice::spawn(Arc::new(VoiceService::new()), bus.clone());
    dash_media::spawn(Arc::new(MediaService::with_demo_tracks()), bus.clone());
    let addr = spawn_gateway(bus.clone()).await;

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    // The UI's Play button sends a raw transcript, exactly like speech would.
    ws.send(Message::Text(
        r#"{ "type": "voice", "transcript": "play music" }"#.into(),
    ))
    .await
    .unwrap();

    let frame = timeout(Duration::from_secs(2), async {
        loop {
            if let Message::Text(text) = ws.next().await.unwrap().unwrap() {
                let v: serde_json::Value = serde_json::from_str(&text).unwrap();
                if v["type"] == "media_state" {
                    return v;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for media_state");

    assert_eq!(frame["playing"], true);
    assert_eq!(frame["track"], "Highway Star");
}

#[tokio::test]
async fn client_set_destination_drives_nav_end_to_end() {
    // client -> gateway -> bus -> nav -> bus -> gateway -> client.
    let bus = Bus::new();
    dash_nav::spawn(Arc::new(NavService::new()), bus.clone());
    let addr = spawn_gateway(bus.clone()).await;

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    ws.send(Message::Text(
        r#"{ "type": "set_destination", "destination": "Pier 39" }"#.into(),
    ))
    .await
    .unwrap();

    let frame = timeout(Duration::from_secs(2), async {
        loop {
            if let Message::Text(text) = ws.next().await.unwrap().unwrap() {
                let v: serde_json::Value = serde_json::from_str(&text).unwrap();
                if v["type"] == "nav_state" {
                    return v;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for nav_state");

    assert_eq!(frame["source"], "nav");
    assert_eq!(frame["destination"], "Pier 39");
}

#[tokio::test]
async fn client_set_setting_drives_settings_end_to_end() {
    // client -> gateway -> bus -> settings -> bus -> gateway -> client.
    let bus = Bus::new();
    dash_settings::spawn(Arc::new(SettingsService::new()), bus.clone());
    let addr = spawn_gateway(bus.clone()).await;

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    // "07" should come back normalized to "7".
    ws.send(Message::Text(
        r#"{ "type": "set_setting", "key": "volume", "value": "07" }"#.into(),
    ))
    .await
    .unwrap();

    let frame = timeout(Duration::from_secs(2), async {
        loop {
            if let Message::Text(text) = ws.next().await.unwrap().unwrap() {
                let v: serde_json::Value = serde_json::from_str(&text).unwrap();
                if v["type"] == "settings_state" {
                    return v;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for settings_state");

    assert_eq!(frame["source"], "settings");
    assert_eq!(frame["key"], "volume");
    assert_eq!(frame["value"], "7");
}

#[tokio::test]
async fn client_command_reaches_the_bus() {
    let bus = Bus::new();
    let addr = spawn_gateway(bus.clone()).await;

    // Observe what the gateway publishes onto the bus.
    let mut probe = bus.subscribe();

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    ws.send(Message::Text(
        r#"{ "type": "set_destination", "destination": "Pier 39" }"#.into(),
    ))
    .await
    .unwrap();

    let event = timeout(Duration::from_secs(2), probe.recv())
        .await
        .expect("timed out")
        .unwrap();
    assert_eq!(event.source, ServiceId::Gateway);
    assert_eq!(
        event.kind,
        EventKind::SetDestination { destination: "Pier 39".into() }
    );
}

#[tokio::test]
async fn malformed_client_message_does_not_kill_the_connection() {
    let bus = Bus::new();
    let addr = spawn_gateway(bus.clone()).await;
    let mut probe = bus.subscribe();

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    // Garbage frame: ignored, not fatal.
    ws.send(Message::Text("not json at all".into())).await.unwrap();
    // A valid command right after must still reach the bus.
    ws.send(Message::Text(
        r#"{ "type": "voice", "transcript": "play" }"#.into(),
    ))
    .await
    .unwrap();

    let event = timeout(Duration::from_secs(2), probe.recv())
        .await
        .expect("connection died after malformed frame")
        .unwrap();
    assert_eq!(
        event.kind,
        EventKind::VoiceCommand { transcript: "play".into() }
    );
}
