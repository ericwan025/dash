//! Integration test: boot the real axum server and echo over a real WebSocket.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Bind the gateway to an ephemeral port, returning the chosen address.
async fn spawn_server() -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, dash_gateway::app()).await.unwrap();
    });
    addr
}

#[tokio::test]
async fn websocket_echoes_text_messages() {
    let addr = spawn_server().await;
    let url = format!("ws://{addr}/ws");

    let (mut ws, _resp) = connect_async(url).await.expect("failed to connect");

    ws.send(Message::Text("play music".into())).await.unwrap();

    let reply = ws.next().await.expect("no reply").expect("ws error");
    assert_eq!(reply, Message::Text("play music".into()));
}

#[tokio::test]
async fn healthz_returns_ok() {
    let addr = spawn_server().await;
    // Minimal raw HTTP/1.1 GET so we don't need an HTTP client dependency.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.starts_with("HTTP/1.1 200"), "unexpected response: {text}");
    assert!(text.trim_end().ends_with("ok"), "body should be ok: {text}");
}
