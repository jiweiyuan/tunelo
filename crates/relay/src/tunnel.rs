//! QUIC tunnel listener — accepts connections from tunelo clients.
//!
//! Flow:
//! 1. Accept QUIC connection
//! 2. Accept control stream → Register → Registered (with random subdomain)
//! 3. Heartbeat loop with TTL countdown
//! 4. Auto-disconnect when TTL expires
//! 5. Cleanup routing table on exit

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::time::{interval, Duration, Instant};
use tracing::{info, info_span, warn, Instrument};

use tunelo_protocol::{
    read_message, write_message, ClientControl, RelayControl, PROTOCOL_VERSION,
};

use crate::router::{Router, TunnelSession};

/// Run the QUIC tunnel listener.
pub async fn run_tunnel_listener(
    addr: String,
    server_config: quinn::ServerConfig,
    router: Arc<Router>,
    domain: String,
    max_ttl: u64,
) -> Result<()> {
    let endpoint = quinn::Endpoint::server(server_config, addr.parse()?)?;
    info!(addr = %addr, max_ttl_secs = max_ttl, "QUIC tunnel listener started");

    while let Some(incoming) = endpoint.accept().await {
        let router = router.clone();
        let domain = domain.clone();
        tokio::spawn(async move {
            let remote = incoming.remote_address();
            async {
                match incoming.await {
                    Ok(conn) => {
                        info!("connected");
                        if let Err(e) =
                            handle_connection(conn, &router, &domain, max_ttl).await
                        {
                            warn!(error = %e, "tunnel ended");
                        }
                    }
                    Err(e) => warn!(error = %e, "accept failed"),
                }
            }
            .instrument(info_span!("tunnel", %remote))
            .await;
        });
    }
    Ok(())
}

/// Handle one tunnel connection: handshake → heartbeat loop with TTL → cleanup.
async fn handle_connection(
    conn: quinn::Connection,
    router: &Router,
    domain: &str,
    max_ttl: u64,
) -> Result<()> {
    let (mut tx, mut rx) = conn.accept_bi().await.context("accept control stream")?;

    // ── Handshake ──────────────────────────────────────────────────────
    let register: ClientControl = read_message(&mut rx).await.context("read Register")?;
    let (version, access_code, requested_ttl) = match register {
        ClientControl::Register {
            version,
            access_code,
            ttl_secs,
        } => (version, access_code, ttl_secs),
        _ => {
            send_error(&mut tx, 1000, "expected Register").await;
            bail!("unexpected first message");
        }
    };

    if version != PROTOCOL_VERSION {
        send_error(
            &mut tx,
            tunelo_protocol::error_codes::VERSION_MISMATCH,
            &format!("version mismatch: server={PROTOCOL_VERSION}, client={version}"),
        )
        .await;
        bail!("version mismatch");
    }

    // Cap TTL to server max
    let granted_ttl = requested_ttl.min(max_ttl);

    // Always assign a random subdomain (no custom subdomains on public relay)
    let subdomain = router.generate_subdomain();
    let hostname = format!("{subdomain}.{domain}");
    let tunnel_id = uuid::Uuid::new_v4().to_string();
    let is_private = access_code.is_some();

    router.register(TunnelSession {
        subdomain: subdomain.clone(),
        hostname: hostname.clone(),
        tunnel_id: tunnel_id.clone(),
        connection: conn.clone(),
        access_code,
    });

    // Ensure cleanup on any exit path
    let _guard = scopeguard::guard((), |_| {
        router.remove(&subdomain);
    });

    write_message(
        &mut tx,
        &RelayControl::Registered {
            hostname: hostname.clone(),
            tunnel_id,
            ttl_secs: granted_ttl,
        },
    )
    .await?;

    let ttl_display = format_duration(granted_ttl);
    info!(hostname = %hostname, is_private, ttl = %ttl_display, "tunnel active");

    // ── Heartbeat loop with TTL ────────────────────────────────────────
    let mut tick = interval(Duration::from_secs(30));
    let deadline = Instant::now() + Duration::from_secs(granted_ttl);

    loop {
        tokio::select! {
            _ = tick.tick() => {
                // Check TTL
                if Instant::now() >= deadline {
                    info!(hostname = %hostname, "TTL expired, shutting down tunnel");
                    let _ = write_message(
                        &mut tx,
                        &RelayControl::Shutdown {
                            reason: format!("Tunnel expired after {ttl_display}"),
                        },
                    ).await;
                    break;
                }
                if write_message(&mut tx, &RelayControl::Heartbeat).await.is_err() {
                    break;
                }
            }
            msg = read_message::<ClientControl, _>(&mut rx) => {
                match msg {
                    Ok(ClientControl::HeartbeatAck) => {}
                    Ok(other) => warn!(?other, "unexpected control message"),
                    Err(_) => break,
                }
            }
            reason = conn.closed() => {
                info!(%reason, "QUIC connection closed");
                break;
            }
        }
    }
    Ok(())
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 && m > 0 {
        format!("{h}h {m}m")
    } else if h > 0 {
        format!("{h}h")
    } else {
        format!("{m}m")
    }
}

async fn send_error(tx: &mut quinn::SendStream, code: u16, msg: &str) {
    let _ = write_message(
        tx,
        &RelayControl::Error {
            code,
            message: msg.into(),
        },
    )
    .await;
}
