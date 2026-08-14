//! QUIC tunnel listener — accepts connections from tunelo clients.
//!
//! - Clients may request a subdomain; falls back to random if taken or absent
//! - Server-side max session duration (silent, not exposed to client)
//! - Heartbeat loop with session timer
//! - Auto-disconnect when session expires

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use rand::RngCore;
use tokio::time::{interval, Duration, Instant};
use tracing::{info, info_span, warn, Instrument};

use tunelo_protocol::{
    error_codes, read_message, subdomain_for_pubkey, verify_ownership, write_message,
    ClientControl, RelayControl, CHALLENGE_NONCE_LEN, PROTOCOL_VERSION,
};

use crate::auth::password_ok;
use crate::proxy;
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

    // The first control message decides the connection's role: a tunnel owner
    // (Register) or a native visitor bridging into an existing tunnel (Attach).
    let hello: ClientControl = read_message(&mut rx).await.context("read control hello")?;
    match hello {
        ClientControl::Register { version, password, requested_subdomain, owner_pubkey } => {
            handle_owner(conn, router, domain, max_session, tx, rx,
                         version, password, requested_subdomain, owner_pubkey).await
        }
        ClientControl::Attach { version, subdomain, password } => {
            handle_visitor(conn, router, tx, version, subdomain, password).await
        }
        _ => {
            send_error(&mut tx, 1000, "expected Register or Attach").await;
            bail!("unexpected first message");
        }
    }
}

/// Serve a tunnel *owner*: register a hostname, then run the heartbeat loop
/// (data streams are opened toward this connection by the visitor paths).
#[allow(clippy::too_many_arguments)]
async fn handle_owner(
    conn: quinn::Connection,
    router: &Router,
    domain: &str,
    max_session: u64,
    mut tx: quinn::SendStream,
    mut rx: quinn::RecvStream,
    version: u8,
    password: Option<String>,
    requested_subdomain: Option<String>,
    owner_pubkey: Option<Vec<u8>>,
) -> Result<()> {
    if version != PROTOCOL_VERSION {
        send_error(
            &mut tx,
            tunelo_protocol::error_codes::VERSION_MISMATCH,
            &format!("version mismatch: server={PROTOCOL_VERSION}, client={version}"),
        )
        .await;
        bail!("version mismatch");
    }

    // A keyed owner claims a subdomain derived from its public key and must
    // prove possession of the private key before the name is granted. A legacy
    // owner gets a requested-or-random name with no proof.
    let subdomain = match &owner_pubkey {
        Some(pubkey) => verify_owner_challenge(&mut tx, &mut rx, pubkey).await?,
        None => router.resolve_subdomain(requested_subdomain.as_deref()),
    };
    let hostname = format!("{subdomain}.{domain}");
    let tunnel_id = uuid::Uuid::new_v4().to_string();
    let is_private = password.is_some();

    router.register(TunnelSession {
        subdomain: subdomain.clone(),
        hostname: hostname.clone(),
        tunnel_id: tunnel_id.clone(),
        connection: conn.clone(),
        password,
        owner_pubkey,
    });

    let guard_subdomain = subdomain.clone();
    let guard_tunnel_id = tunnel_id.clone();
    let _guard = scopeguard::guard((), move |_| {
        router.remove_if(&guard_subdomain, &guard_tunnel_id);
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

/// Serve a native *visitor*: authenticate against the target tunnel, then
/// bridge every bidi stream the visitor opens to the owner connection.
async fn handle_visitor(
    conn: quinn::Connection,
    router: &Router,
    mut tx: quinn::SendStream,
    version: u8,
    subdomain: String,
    password: Option<String>,
) -> Result<()> {
    if version != PROTOCOL_VERSION {
        send_error(
            &mut tx,
            error_codes::VERSION_MISMATCH,
            &format!("version mismatch: server={PROTOCOL_VERSION}, client={version}"),
        )
        .await;
        bail!("visitor version mismatch");
    }

    let session = match router.get(&subdomain) {
        Some(s) => s,
        None => {
            send_error(&mut tx, error_codes::NO_SUCH_TUNNEL, &format!("no tunnel: {subdomain}")).await;
            bail!("attach to unknown tunnel {subdomain}");
        }
    };

    if !password_ok(session.password.as_deref(), password.as_deref()) {
        send_error(&mut tx, error_codes::UNAUTHORIZED, "invalid tunnel password").await;
        bail!("visitor unauthorized for {subdomain}");
    }

    write_message(
        &mut tx,
        &RelayControl::Attached {
            hostname: session.hostname.clone(),
            tunnel_id: session.tunnel_id.clone(),
        },
    )
    .await?;
    info!(subdomain = %subdomain, "visitor attached");

    // Bridge streams until the visitor disconnects or the owner tunnel dies.
    loop {
        tokio::select! {
            accepted = conn.accept_bi() => {
                let (v_send, v_recv) = match accepted {
                    Ok(s) => s,
                    Err(e) => {
                        info!(error = %e, "visitor stream accept ended");
                        break;
                    }
                };
                // Re-read the session each stream so a reconnected owner (same
                // subdomain, new QUIC connection) is picked up without dropping
                // the visitor.
                let Some(session) = router.get(&subdomain) else {
                    info!(subdomain = %subdomain, "owner tunnel gone, dropping visitor");
                    break;
                };
                tokio::spawn(
                    async move {
                        if let Err(e) = proxy::bridge_visitor_stream(&session, v_send, v_recv).await {
                            warn!(error = %e, "visitor bridge error");
                        }
                    }
                    .instrument(info_span!("bridge")),
                );
            }
            reason = conn.closed() => {
                info!(%reason, "visitor connection closed");
                break;
            }
        }
    }
    Ok(())
}

/// Challenge a keyed owner to prove possession of its private key. Returns the
/// subdomain derived from the public key on success; errors (after telling the
/// client) if the proof is missing or the signature does not verify.
async fn verify_owner_challenge(
    tx: &mut quinn::SendStream,
    rx: &mut quinn::RecvStream,
    pubkey: &[u8],
) -> Result<String> {
    let subdomain = subdomain_for_pubkey(pubkey);

    let mut nonce = vec![0u8; CHALLENGE_NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    write_message(tx, &RelayControl::Challenge { nonce: nonce.clone() }).await?;

    let proof: ClientControl = read_message(rx).await.context("read ownership proof")?;
    let ClientControl::Prove { signature } = proof else {
        send_error(tx, error_codes::IDENTITY_INVALID, "expected ownership proof").await;
        bail!("expected Prove message during ownership challenge");
    };

    if !verify_ownership(pubkey, &nonce, &signature) {
        send_error(tx, error_codes::IDENTITY_INVALID, "invalid ownership proof").await;
        bail!("invalid ownership proof for {subdomain}");
    }

    Ok(subdomain)
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
