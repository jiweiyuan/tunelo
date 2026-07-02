//! Protocol messages exchanged between client and relay.

use serde::{Deserialize, Serialize};

/// Messages sent from the client to the relay on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientControl {
    /// Initial registration request. Opens a tunnel and receives a public
    /// hostname; the connection then serves data streams (the tunnel *owner*).
    Register {
        /// Protocol version for compatibility checking.
        version: u8,
        /// Optional password for private tunnels.
        #[serde(default, alias = "access_code")]
        password: Option<String>,
        /// Request a specific subdomain (e.g. for reconnect stability).
        #[serde(default)]
        requested_subdomain: Option<String>,
    },
    /// Attach to an existing tunnel as a native *visitor*, bridging QUIC
    /// streams straight to the owner instead of arriving over HTTP. Lets a
    /// native client (e.g. a mobile app) reach the owner's local service with
    /// QUIC end to end — multiplexing, loss recovery, no HTTP framing.
    ///
    /// After the relay replies with [`RelayControl::Attached`], every bidi
    /// stream the visitor opens is paired to a fresh stream on the owner, so
    /// the visitor can run any byte protocol (a WebSocket upgrade, a raw
    /// framed stream, etc.) over it.
    Attach {
        /// Protocol version for compatibility checking.
        version: u8,
        /// Target tunnel subdomain, e.g. "swift-fox-3847".
        subdomain: String,
        /// Password for private tunnels; ignored for public ones.
        #[serde(default)]
        password: Option<String>,
    },
    /// Response to a heartbeat ping from the relay.
    HeartbeatAck,
}

/// Messages sent from the relay to the client on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelayControl {
    /// Successful registration response.
    Registered {
        /// The full public hostname, e.g. "abc123.tunelo.net"
        hostname: String,
        /// Unique tunnel session ID.
        tunnel_id: String,
    },
    /// Successful attach response to a [`ClientControl::Attach`]. Subsequent
    /// bidi streams opened by the visitor are bridged to the owner.
    Attached {
        /// The tunnel's public hostname, echoed back for confirmation.
        hostname: String,
        /// The owner tunnel's session ID.
        tunnel_id: String,
    },
    /// Registration or protocol error.
    Error { code: u16, message: String },
    /// Periodic heartbeat to verify the tunnel is alive.
    Heartbeat,
    /// Server-initiated shutdown of the tunnel.
    Shutdown { reason: String },
}

// ─── Error Codes ─────────────────────────────────────────────────────────────

pub mod error_codes {
    pub const SUBDOMAIN_TAKEN: u16 = 1001;
    pub const INVALID_SUBDOMAIN: u16 = 1002;
    pub const VERSION_MISMATCH: u16 = 1003;
    pub const SERVER_FULL: u16 = 1004;
    /// Attach target subdomain has no live tunnel.
    pub const NO_SUCH_TUNNEL: u16 = 1005;
    /// Attach password missing or incorrect for a private tunnel.
    pub const UNAUTHORIZED: u16 = 1006;

    pub const INTERNAL_ERROR: u16 = 1500;
}
