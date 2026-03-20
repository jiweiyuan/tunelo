//! Protocol messages exchanged between client and relay.

use serde::{Deserialize, Serialize};

/// Messages sent from the client to the relay on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientControl {
    /// Initial registration request.
    Register {
        /// Protocol version for compatibility checking.
        version: u8,
        /// Optional access code for private tunnels.
        #[serde(default)]
        access_code: Option<String>,
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

    pub const INTERNAL_ERROR: u16 = 1500;
}
