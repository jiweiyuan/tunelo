//! Tunelo Relay — public-facing server.
//!
//! 1. Accepts QUIC tunnel connections from clients
//! 2. Accepts public HTTP connections from browsers
//! 3. Routes by hostname through the tunnel

pub mod http_listener;
pub mod proxy;
pub mod router;
pub mod tls;
pub mod tunnel;

use std::sync::Arc;
use anyhow::Result;
use tracing::info;

/// Start the relay server.
pub async fn run(domain: String, tunnel_addr: String, http_addr: String, max_ttl: u64) -> Result<()> {
    info!(domain = %domain, tunnel = %tunnel_addr, http = %http_addr, max_ttl_secs = max_ttl, "starting relay");

    let router = Arc::new(router::Router::new());
    let quic_config = tls::build_quic_server_config()?;

    let r1 = router.clone();
    let t_addr = tunnel_addr.clone();
    let d = domain.clone();
    let tunnel_task = tokio::spawn(async move {
        tunnel::run_tunnel_listener(t_addr, quic_config, r1, d, max_ttl).await
    });

    let r2 = router.clone();
    let h_addr = http_addr.clone();
    let http_task = tokio::spawn(async move {
        http_listener::run_http_listener(h_addr, r2).await
    });

    tokio::select! {
        r = tunnel_task => r??,
        r = http_task => r??,
    }
    Ok(())
}
