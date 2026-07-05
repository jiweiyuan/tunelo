//! Stable, self-owned subdomains derived from an Ed25519 public key.
//!
//! The design mirrors how Syncthing derives a device ID from a certificate and
//! how Tailscale identifies a node by a keypair rather than a hardware id: the
//! public subdomain is a hash of the owner's public key, and only the holder of
//! the matching private key can prove ownership by signing the relay's nonce.
//! Because the name is a one-way hash of the key, it is non-enumerable and
//! effectively unstealable — no shared secret and no relay-side state required.

use sha2::{Digest, Sha256};

/// Domain-separation tag so the subdomain hash can never collide with any other
/// use of the key material.
const NAME_TAG: &[u8] = b"tunelo-name-v1:";

/// Length of the ownership challenge nonce, in bytes.
pub const CHALLENGE_NONCE_LEN: usize = 32;

/// Derive the DNS-safe subdomain label for an Ed25519 public key.
///
/// Returns 16 lowercase hex characters (64 bits) — a valid DNS label that is
/// infeasible to enumerate. Deterministic: the same key always yields the same
/// name across restarts, reinstalls, and networks.
pub fn subdomain_for_pubkey(pubkey: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(NAME_TAG);
    hasher.update(pubkey);
    let digest = hasher.finalize();
    hex_lower(&digest[..8])
}

/// Verify that `signature` over `nonce` was produced by the private key
/// matching `pubkey`. Any malformed input verifies as `false` rather than
/// erroring, so callers can treat verification as a simple boolean gate.
pub fn verify_ownership(pubkey: &[u8], nonce: &[u8], signature: &[u8]) -> bool {
    use ed25519_dalek::{Signature, VerifyingKey};

    let Ok(key_bytes) = <[u8; 32]>::try_from(pubkey) else {
        return false;
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&key_bytes) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(signature) else {
        return false;
    };
    verifying_key.verify_strict(nonce, &sig).is_ok()
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[test]
    fn subdomain_is_stable_and_dns_safe() {
        let pubkey = key().verifying_key().to_bytes();
        let a = subdomain_for_pubkey(&pubkey);
        let b = subdomain_for_pubkey(&pubkey);
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert!(a.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn valid_signature_verifies() {
        let signing = key();
        let pubkey = signing.verifying_key().to_bytes();
        let nonce = [42u8; CHALLENGE_NONCE_LEN];
        let sig = signing.sign(&nonce).to_bytes();
        assert!(verify_ownership(&pubkey, &nonce, &sig));
    }

    #[test]
    fn wrong_nonce_or_key_fails() {
        let signing = key();
        let pubkey = signing.verifying_key().to_bytes();
        let nonce = [42u8; CHALLENGE_NONCE_LEN];
        let sig = signing.sign(&nonce).to_bytes();

        assert!(!verify_ownership(&pubkey, &[9u8; CHALLENGE_NONCE_LEN], &sig));
        let other = SigningKey::from_bytes(&[8u8; 32]).verifying_key().to_bytes();
        assert!(!verify_ownership(&other, &nonce, &sig));
        assert!(!verify_ownership(&pubkey, &nonce, &[0u8; 64]));
    }
}
