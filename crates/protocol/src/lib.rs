pub mod codec;
pub mod identity;
pub mod messages;

pub use codec::{read_message, write_message};
pub use identity::{subdomain_for_pubkey, verify_ownership, CHALLENGE_NONCE_LEN};
pub use messages::*;

/// Protocol version — increment on breaking changes.
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum control message size (64 KB).
pub const MAX_MESSAGE_SIZE: u32 = 65536;

/// Size of the relay copy buffer. 64 KB matches typical kernel socket buffers
/// and is the sweet spot for throughput vs memory on modern systems.
pub const RELAY_BUF_SIZE: usize = 64 * 1024;
