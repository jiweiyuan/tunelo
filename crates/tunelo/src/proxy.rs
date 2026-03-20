//! Data-plane relay — bidirectional byte copy between QUIC and localhost.
//!
//! ZERO parsing. Raw bytes in, raw bytes out.
//! Same pattern as bore's copy_bidirectional, over QUIC streams.

use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::{Context as _, Result};
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tracing::debug;

/// Relay one data stream to localhost and back.
pub async fn handle_data_stream(
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    local_addr: &str,
) -> Result<()> {
    let mut local = TcpStream::connect(local_addr)
        .await
        .context("connect local")?;
    local.set_nodelay(true)?;

    let mut tunnel = QuicBidi { send, recv };

    match io::copy_bidirectional(&mut local, &mut tunnel).await {
        Ok((up, down)) => debug!(up, down, "relay done"),
        Err(e) => debug!(error = %e, "relay ended"),
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
