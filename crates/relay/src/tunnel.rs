//! QUIC tunnel listener — accepts connections from tunelo clients.
//!
//! - Random subdomain only (no custom subdomains)
//! - Server-side max session duration (silent, not exposed to client)
//! - Heartbeat loop with session timer
//! - Auto-disconnect when session expires

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
    max_session: u64,
) -> Result<()> {
    let endpoint = quinn::Endpoint::server(server_config, addr.parse()?)?;
    let session_display = if max_session == 0 { "unlimited".into() } else { format_duration(max_session) };
    info!(addr = %addr, max_session = %session_display, "QUIC tunnel listener started");

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
                            handle_connection(conn, &router, &domain, max_session).await
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

async fn handle_connection(
    conn: quinn::Connection,
    router: &Router,
    domain: &str,
    max_session: u64,
) -> Result<()> {
    let (mut tx, mut rx) = conn.accept_bi().await.context("accept control stream")?;

    // ── Handshake ──────────────────────────────────────────────────────
    let register: ClientControl = read_message(&mut rx).await.context("read Register")?;
    let (version, access_code) = match register {
        ClientControl::Register { version, access_code } => (version, access_code),
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

    // Always random subdomain
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

    let _guard = scopeguard::guard((), |_| {
        router.remove(&subdomain);
    });

    write_message(
        &mut tx,
        &RelayControl::Registered {
            hostname: hostname.clone(),
            tunnel_id,
        },
    )
    .await?;
    info!(hostname = %hostname, is_private, "tunnel active");

    // ── Heartbeat loop with session timer ──────────────────────────────
    let mut tick = interval(Duration::from_secs(30));
    let deadline = if max_session > 0 {
        Some(Instant::now() + Duration::from_secs(max_session))
    } else {
        None
    };

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if let Some(dl) = deadline {
                    if Instant::now() >= dl {
                        let dur = format_duration(max_session);
                        info!(hostname = %hostname, "session expired after {dur}");
                        let _ = write_message(
                            &mut tx,
                            &RelayControl::Shutdown {
                                reason: format!("Session expired ({dur}). Reconnect to start a new one."),
                            },
                        ).await;
                        break;
                    }
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
    if h > 0 && m > 0 { format!("{h}h{m}m") }
    else if h > 0 { format!("{h}h") }
    else { format!("{m}m") }
}

async fn send_error(tx: &mut quinn::SendStream, code: u16, msg: &str) {
    let _ = write_message(
        tx,
        &RelayControl::Error { code, message: msg.into() },
    )
    .await;
}
