//! # dash-gateway
//!
//! The bridge between the internal [`Bus`](dash_bus::Bus) and WebSocket clients
//! (the Flutter frontend).
//!
//! Each accepted WebSocket connection runs two concurrent halves:
//!
//! - **Outbound** — subscribes to the bus and forwards every [`Event`] to the
//!   client as a flattened [`ServerEvent`] JSON frame.
//! - **Inbound** — parses each client text frame as a [`ClientCommand`] and
//!   publishes the resulting [`Event`](dash_core::Event) onto the bus.
//!
//! The bus does the rest: a command published here is picked up by whichever
//! service cares (e.g. media), which publishes its new state, which the outbound
//! half then streams back to every connected client.
//!
//! The [`Bus`] is shared into the router as axum state, so [`app`] can be bound
//! by both `main` and integration tests.

pub mod protocol;

pub use protocol::{ClientCommand, ServerEvent};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use dash_bus::Bus;
use dash_core::CoreError;
use futures_util::{SinkExt, StreamExt};

/// Build the gateway's HTTP router, sharing `bus` with every connection.
///
/// Routes:
/// - `GET /healthz` → `"ok"`.
/// - `GET /ws` → upgrades to a WebSocket bridged to the bus.
pub fn app(bus: Bus) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .with_state(bus)
}

/// Upgrade an incoming request and bridge the socket to the bus.
async fn ws_handler(ws: WebSocketUpgrade, State(bus): State<Bus>) -> Response {
    ws.on_upgrade(move |socket| bridge_socket(socket, bus))
}

/// Run the outbound and inbound halves of one client connection until either
/// side closes, then tear the other down.
async fn bridge_socket(socket: WebSocket, bus: Bus) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Outbound: bus events -> client JSON frames.
    let mut sub = bus.subscribe();
    let mut outbound = tokio::spawn(async move {
        loop {
            match sub.recv().await {
                Ok(event) => {
                    let frame = match serde_json::to_string(&ServerEvent::from(event)) {
                        Ok(json) => json,
                        Err(e) => {
                            eprintln!("[gateway] failed to serialize event: {e}");
                            continue;
                        }
                    };
                    if ws_tx.send(Message::Text(frame.into())).await.is_err() {
                        break; // client disconnected
                    }
                }
                // A lagging client just skips ahead; keep the connection alive.
                Err(CoreError::Lagged(n)) => {
                    eprintln!("[gateway] client lagged, skipped {n} event(s)");
                }
                Err(_) => break, // bus closed
            }
        }
    });

    // Inbound: client JSON frames -> bus events.
    let inbound_bus = bus.clone();
    let mut inbound = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<ClientCommand>(&text) {
                    Ok(cmd) => {
                        inbound_bus.publish(cmd.into_event());
                    }
                    Err(e) => eprintln!("[gateway] ignoring malformed client message: {e}"),
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // When either half ends, cancel the other so the task set drains.
    tokio::select! {
        _ = &mut outbound => inbound.abort(),
        _ = &mut inbound => outbound.abort(),
    }
}
