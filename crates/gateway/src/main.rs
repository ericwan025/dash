//! Binary entry point for the dash gateway.
//!
//! Wires up the shared [`Bus`](dash_bus::Bus), starts the services that run on
//! it (currently media), and serves the WebSocket bridge.
//!
//! Bind address defaults to `127.0.0.1:8080`; override with `DASH_GATEWAY_ADDR`.

use dash_bus::Bus;
use dash_media::MediaService;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = std::env::var("DASH_GATEWAY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;

    // The single bus every service and every client connection shares.
    let bus = Bus::new();

    // Start the media service loop on the bus.
    let media = Arc::new(MediaService::with_demo_tracks());
    dash_media::spawn(media, bus.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("dash-gateway listening on ws://{addr}/ws  (health: http://{addr}/healthz)");

    axum::serve(listener, dash_gateway::app(bus)).await?;
    Ok(())
}
