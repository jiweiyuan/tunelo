//! Persistent client identity for stable, self-owned subdomains.
//!
//! On first use an Ed25519 keypair is generated and saved; thereafter the same
//! key — and therefore the same subdomain — is reused across restarts. The key
//! is the sole owner credential: the relay derives the subdomain from the
//! public key and challenges the client to sign a nonce, so only the holder of
//! this private key can claim the name. See `tunelo_protocol::identity`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use rand::RngCore;

/// A loaded (or freshly created) tunnel identity.
pub struct Identity {
    signing_key: SigningKey,
}

impl Identity {
    /// Load the identity from `path`, or from the default location when `None`,
    /// generating and persisting a new keypair if none exists yet.
    pub fn load_or_create(path: Option<PathBuf>) -> Result<Self> {
        let path = match path {
            Some(p) => p,
            None => default_key_path()?,
        };

        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(seed) = <[u8; 32]>::try_from(bytes.as_slice()) {
                return Ok(Self {
                    signing_key: SigningKey::from_bytes(&seed),
                });
            }
            // A file exists but is not a 32-byte seed — refuse to clobber it.
            anyhow::bail!("identity file {} is corrupt (expected 32 bytes)", path.display());
        }

        let mut seed = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        persist(&path, &seed)
            .with_context(|| format!("save identity to {}", path.display()))?;
        Ok(Self { signing_key })
    }

    /// The 32-byte Ed25519 public key sent to the relay in `Register`.
    pub fn public_key(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }

    /// The stable subdomain label this identity owns.
    pub fn subdomain(&self) -> String {
        tunelo_protocol::subdomain_for_pubkey(&self.public_key())
    }

    /// Sign a relay challenge nonce, proving ownership.
    pub fn sign(&self, nonce: &[u8]) -> Vec<u8> {
        self.signing_key.sign(nonce).to_bytes().to_vec()
    }
}

fn default_key_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("could not determine a config directory for the identity key")?
        .join("tunelo");
    Ok(dir.join("identity.key"))
}

fn persist(path: &Path, seed: &[u8; 32]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, seed)?;
    restrict_permissions(path);
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}
