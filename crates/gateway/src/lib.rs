//! # dash-gateway
//!
//! The bridge between the internal [`Bus`](dash_bus::Bus) and a WebSocket client
//! (the Flutter frontend).
//!
//! At this stage the gateway is a plain **echo** server: it accepts a WebSocket
//! connection on `/ws` and sends every text message straight back. That proves
//! the transport works end to end — an axum server accepting a real client
//! connection — before we layer the bus ↔ JSON translation on top.
//!
//! The router is built in [`app`] (rather than inline in `main`) so integration
//! tests can bind it to an ephemeral port and connect a real client.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};

/// Build the gateway's HTTP router.
///
/// Routes:
/// - `GET /healthz` → `"ok"`, a trivial liveness check.
/// - `GET /ws` → upgrades to a WebSocket that currently echoes messages.
pub fn app() -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
}

/// Upgrade an incoming HTTP request to a WebSocket connection.
async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(echo_socket)
}

/// Echo loop: send every received text/binary message back to the client until
/// the socket closes or errors.
async fn echo_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if socket.send(Message::Text(text)).await.is_err() {
                    break; // client went away
                }
            }
            Message::Binary(bytes) => {
                if socket.send(Message::Binary(bytes)).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            // Ping/Pong are handled by axum's WebSocket automatically.
            _ => {}
        }
    }
}
