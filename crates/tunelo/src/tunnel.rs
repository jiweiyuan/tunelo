//! Tunnel connection management — QUIC connection to the relay.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tracing::{error, info, info_span, warn, Instrument};

use tunelo_protocol::{
    read_message, write_message, ClientControl, RelayControl, PROTOCOL_VERSION,
};

use crate::proxy;

/// Establish tunnel and serve requests.
pub async fn run_tunnel(
    port: u16,
    local_host: String,
    relay: String,
    access_code: Option<String>,
) -> Result<()> {
    let conn = connect(&relay).await?;
    info!(relay = %relay, "connected");

    // ── Handshake ──────────────────────────────────────────────────────
    let (mut tx, mut rx) = conn.open_bi().await?;
    write_message(
        &mut tx,
        &ClientControl::Register {
            version: PROTOCOL_VERSION,
            access_code: access_code.clone(),
        },
    )
    .await?;

    let resp: RelayControl = read_message(&mut rx).await?;
    let (hostname, tunnel_id) = match resp {
        RelayControl::Registered { hostname, tunnel_id } => (hostname, tunnel_id),
        RelayControl::Error { code, message } => {
            bail!("relay error ({code}): {message}");
        }
        _ => bail!("unexpected response"),
    };

    // ── Print the URL ──────────────────────────────────────────────────
    println!();
    println!("  \x1b[32m✔\x1b[0m Tunnel is ready!");
    println!();
    if let Some(ref code) = access_code {
        println!("  Share URL:   \x1b[1;36mhttps://{hostname}?pwd={code}\x1b[0m");
        println!("  \x1b[33m🔒 Private\x1b[0m — visitors without the link will be asked for the code");
    } else {
        println!("  Public URL:  \x1b[1;36mhttps://{hostname}\x1b[0m");
    }
    println!("  Forwarding:  → http://{local_host}:{port}");
    println!("  Tunnel ID:   {tunnel_id}");
    println!();

    // ── Run ────────────────────────────────────────────────────────────
    let local_addr: Arc<str> = format!("{local_host}:{port}").into();

    let conn2 = conn.clone();
    let addr2 = local_addr.clone();
    let data_handle = tokio::spawn(async move {
        loop {
            let (send, recv) = match conn2.accept_bi().await {
                Ok(s) => s,
                Err(e) => {
                    info!(error = %e, "data accept ended");
                    return;
                }
            };
            let addr = addr2.clone();
            tokio::spawn(
                async move {
                    if let Err(e) = proxy::handle_data_stream(send, recv, &addr).await {
                        warn!(error = %e, "stream error");
                    }
                }
                .instrument(info_span!("req")),
            );
        }
    });

    // Control loop: heartbeats + server-initiated shutdown
    loop {
        match read_message::<RelayControl, _>(&mut rx).await {
            Ok(RelayControl::Heartbeat) => {
                let _ = write_message(&mut tx, &ClientControl::HeartbeatAck).await;
            }
            Ok(RelayControl::Shutdown { reason }) => {
                info!(%reason, "relay shutdown");
                println!("\n  \x1b[33m⏱\x1b[0m  {reason}");
                break;
            }
            Ok(RelayControl::Error { code, message }) => {
                error!(code, %message, "relay error");
                break;
            }
            Ok(_) => {}
            Err(_) => {
                info!("control stream closed");
                break;
            }
        }
    }

    data_handle.abort();
    println!("  Tunnel closed.");
    Ok(())
}

/// Connect to the relay via QUIC.
async fn connect(relay: &str) -> Result<quinn::Connection> {
    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureVerifier))
        .with_no_client_auth();
    crypto.alpn_protocols = vec![b"tunelo/1".to_vec()];

    let mut client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    ));

    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(quinn::IdleTimeout::try_from(
        std::time::Duration::from_secs(300),
    )?));
    transport.keep_alive_interval(Some(std::time::Duration::from_secs(15)));
    transport.max_concurrent_bidi_streams(4096u32.into());
    client_config.transport_config(Arc::new(transport));

    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    let addr: std::net::SocketAddr = relay
        .parse()
        .or_else(|_| {
            use std::net::ToSocketAddrs;
            relay
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| std::io::Error::other("no addresses"))
        })
        .context("resolve relay")?;

    endpoint
        .connect(addr, "tunelo")?
        .await
        .context("QUIC connect")
}

#[derive(Debug)]
struct InsecureVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self, _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>, _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
