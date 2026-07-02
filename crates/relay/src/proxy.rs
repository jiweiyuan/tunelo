//! Data-plane relay — streams raw HTTP bytes through the QUIC tunnel.
//!
//! ZERO parsing on the data path. The relay already extracted the Host
//! header before calling into this module. Everything here is a raw byte
//! pipe: public TCP socket ↔ QUIC stream ↔ client ↔ localhost.
//!
//! Inspired by bore's copy_bidirectional and rathole's zero-copy forwarding,
//! but over multiplexed QUIC streams instead of per-request TCP connections.

use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::{Context as _, Result};
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};
use tracing::debug;

use crate::router::TunnelSession;

/// Relay a public TCP connection through the tunnel.
///
/// Opens a QUIC bidi stream, then copy_bidirectional:
///   browser ↔ QUIC stream ↔ client ↔ localhost
pub async fn relay_connection(
    session: &TunnelSession,
    mut public_stream: tokio::net::TcpStream,
) -> Result<()> {
    let (send, recv) = session
        .connection
        .open_bi()
        .await
        .context("open QUIC stream")?;

    debug!(subdomain = %session.subdomain, "relaying");

    let mut tunnel = QuicBidi { send, recv };

    match io::copy_bidirectional(&mut public_stream, &mut tunnel).await {
        Ok((up, down)) => debug!(up, down, "relay done"),
        Err(e) => debug!(error = %e, "relay ended"),
    }
    Ok(())
}

/// Bridge a native visitor's QUIC stream to the owner tunnel.
///
/// Opens a fresh bidi stream on the owner connection and pipes bytes both
/// ways between it and the visitor's stream — the QUIC-to-QUIC analogue of
/// [`relay_connection`], with no TCP hop and no HTTP framing on either side.
pub async fn bridge_visitor_stream(
    session: &TunnelSession,
    visitor_send: quinn::SendStream,
    visitor_recv: quinn::RecvStream,
) -> Result<()> {
    let (owner_send, owner_recv) = session
        .connection
        .open_bi()
        .await
        .context("open owner QUIC stream")?;

    debug!(subdomain = %session.subdomain, "bridging visitor stream");

    let mut visitor = QuicBidi { send: visitor_send, recv: visitor_recv };
    let mut owner = QuicBidi { send: owner_send, recv: owner_recv };

    match io::copy_bidirectional(&mut visitor, &mut owner).await {
        Ok((up, down)) => debug!(up, down, "visitor bridge done"),
        Err(e) => debug!(error = %e, "visitor bridge ended"),
    }
    Ok(())
}

/// Combines QUIC Send+Recv streams into a single AsyncRead+AsyncWrite.
struct QuicBidi {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
}

impl AsyncRead for QuicBidi {
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // quinn::RecvStream implements tokio::io::AsyncRead
        Pin::new(&mut self.get_mut().recv).poll_read(cx, buf)
    }
}

impl AsyncWrite for QuicBidi {
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // quinn::SendStream implements tokio::io::AsyncWrite (returns io::Error)
        <quinn::SendStream as AsyncWrite>::poll_write(
            Pin::new(&mut self.get_mut().send),
            cx,
            buf,
        )
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        <quinn::SendStream as AsyncWrite>::poll_flush(
            Pin::new(&mut self.get_mut().send),
            cx,
        )
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        <quinn::SendStream as AsyncWrite>::poll_shutdown(
            Pin::new(&mut self.get_mut().send),
            cx,
        )
    }
}
