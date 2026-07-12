//! Binary entry point for the dash gateway.
//!
//! Binds the [`app`](dash_gateway::app) router to an address (default
//! `127.0.0.1:8080`, override with the `DASH_GATEWAY_ADDR` env var) and serves
//! until interrupted.

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = std::env::var("DASH_GATEWAY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("dash-gateway listening on ws://{addr}/ws  (health: http://{addr}/healthz)");

    axum::serve(listener, dash_gateway::app()).await?;
    Ok(())
}
