//! Binary entry point for the dash gateway.
//!
//! Wires up the shared [`Bus`](dash_bus::Bus), starts the services that run on
//! it (currently media), and serves the WebSocket bridge.
//!
//! Bind address defaults to `127.0.0.1:8080`; override with `DASH_GATEWAY_ADDR`.

use dash_bus::Bus;
use dash_media::MediaService;
use dash_nav::NavService;
use dash_settings::SettingsService;
use dash_voice::VoiceService;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = std::env::var("DASH_GATEWAY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;

    // The single bus every service and every client connection shares.
    let bus = Bus::new();

    // Start every service on the shared bus.
    // Voice turns raw transcripts into structured commands; the domain services
    // react to those commands and publish their new state.
    dash_voice::spawn(Arc::new(VoiceService::new()), bus.clone());
    dash_media::spawn(Arc::new(MediaService::with_demo_tracks()), bus.clone());
    dash_nav::spawn(Arc::new(NavService::new()), bus.clone());
    dash_settings::spawn(Arc::new(SettingsService::new()), bus.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("dash-gateway listening on ws://{addr}/ws  (health: http://{addr}/healthz)");

    axum::serve(listener, dash_gateway::app(bus)).await?;
    Ok(())
}
