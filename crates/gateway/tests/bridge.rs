//! Integration test: a real client sends a command through the gateway, the
//! media service (running on the same bus) reacts, and the resulting state is
//! streamed back over the WebSocket.

use dash_bus::Bus;
use dash_media::MediaService;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Boot a gateway whose bus also has the media service running on it.
async fn spawn_gateway_with_media() -> std::net::SocketAddr {
    let bus = Bus::new();
    dash_media::spawn(Arc::new(MediaService::with_demo_tracks()), bus.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, dash_gateway::app(bus)).await.unwrap();
    });
    addr
}

#[tokio::test]
async fn client_voice_command_flows_through_to_media_state() {
    let addr = spawn_gateway_with_media().await;
    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();

    // User taps "play": the button sends a voice command.
    ws.send(Message::Text(
        r#"{ "type": "voice", "transcript": "play music" }"#.into(),
    ))
    .await
    .unwrap();

    // Expect a media_state frame with playing = true to come back.
    let frame = timeout(Duration::from_secs(2), async {
        loop {
            let msg = ws.next().await.expect("closed").expect("ws error");
            if let Message::Text(text) = msg {
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
async fn malformed_client_message_does_not_kill_the_connection() {
    let addr = spawn_gateway_with_media().await;
    let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();

    // Garbage frame: must be ignored, not fatal.
    ws.send(Message::Text("not json at all".into())).await.unwrap();
    // A valid command right after should still work.
    ws.send(Message::Text(
        r#"{ "type": "voice", "transcript": "play" }"#.into(),
    ))
    .await
    .unwrap();

    let got = timeout(Duration::from_secs(2), async {
        loop {
            let msg = ws.next().await.expect("closed").expect("ws error");
            if let Message::Text(text) = msg {
                let v: serde_json::Value = serde_json::from_str(&text).unwrap();
                if v["type"] == "media_state" {
                    return v["playing"].as_bool().unwrap();
                }
            }
        }
    })
    .await
    .expect("connection died after malformed frame");

    assert!(got);
}
