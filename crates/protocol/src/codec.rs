//! Length-prefixed message codec for control stream messages.
//!
//! Frame format: [4-byte big-endian length] [msgpack payload]
//!
//! Only used on the control stream (low frequency).
//! Data streams use raw byte relay with zero framing overhead.

use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::MAX_MESSAGE_SIZE;

/// Write a length-prefixed msgpack message to a stream.
#[inline]
pub async fn write_message<T: Serialize, W: AsyncWrite + Unpin>(
    writer: &mut W,
    msg: &T,
) -> Result<()> {
    let payload = rmp_serde::to_vec_named(msg).context("serialize failed")?;
    let len = payload.len() as u32;
    if len > MAX_MESSAGE_SIZE {
        bail!("message too large: {len} bytes (max {MAX_MESSAGE_SIZE})");
    }
    // Write length + payload in a single syscall via a combined buffer
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&payload);
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

/// Read a length-prefixed msgpack message from a stream.
#[inline]
pub async fn read_message<T: DeserializeOwned, R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<T> {
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .context("failed to read message length")?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_MESSAGE_SIZE {
        bail!("message too large: {len} bytes (max {MAX_MESSAGE_SIZE})");
    }
    let mut payload = vec![0u8; len as usize];
    reader
        .read_exact(&mut payload)
        .await
        .context("failed to read message payload")?;
    rmp_serde::from_slice(&payload).context("deserialize failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ClientControl;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_roundtrip() {
        let msg = ClientControl::Register {
            version: 1,
            password: None,
            requested_subdomain: None,
        };
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).await.unwrap();

        let mut cursor = Cursor::new(buf);
        let decoded: ClientControl = read_message(&mut cursor).await.unwrap();
        match decoded {
            ClientControl::Register {
                version,
                password,
                ..
            } => {
                assert_eq!(version, 1);
                assert!(password.is_none());
            }
            _ => panic!("unexpected message type"),
        }
    }

    #[tokio::test]
    async fn test_roundtrip_with_password() {
        let msg = ClientControl::Register {
            version: 1,
            password: Some("fox4217".into()),
            requested_subdomain: None,
        };
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).await.unwrap();

        let mut cursor = Cursor::new(buf);
        let decoded: ClientControl = read_message(&mut cursor).await.unwrap();
        match decoded {
            ClientControl::Register { password, .. } => {
                assert_eq!(password.as_deref(), Some("fox4217"));
            }
            _ => panic!("unexpected message type"),
        }
    }
}
