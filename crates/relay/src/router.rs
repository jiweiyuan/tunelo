//! Hostname → tunnel routing table.
//!
//! Inspired by frp's vhost router (domain-indexed map) and
//! cloudflared's "hostname bound to live tunnel" model.
//!
//! Uses DashMap for lock-free concurrent reads on the hot path.
//! Generates Docker/Heroku-style human-readable subdomain names
//! like "swift-fox" or "calm-river" via the `names` crate.

use dashmap::DashMap;
use names::Generator;
use quinn::Connection;
use tracing::info;

/// An active tunnel session.
#[derive(Clone)]
pub struct TunnelSession {
    pub subdomain: String,
    pub hostname: String,
    pub tunnel_id: String,
    pub connection: Connection,
    /// Optional password for private tunnels.
    pub password: Option<String>,
}

/// Thread-safe hostname → tunnel router.
pub struct Router {
    sessions: DashMap<String, TunnelSession>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    /// Register a tunnel session.
    #[inline]
    pub fn register(&self, session: TunnelSession) {
        info!(subdomain = %session.subdomain, hostname = %session.hostname, "tunnel registered");
        self.sessions.insert(session.subdomain.clone(), session);
    }

    /// Look up a tunnel session by subdomain.
    #[inline]
    pub fn get(&self, subdomain: &str) -> Option<TunnelSession> {
        self.sessions.get(subdomain).map(|r| r.value().clone())
    }

    /// Remove a tunnel session.
    #[inline]
    pub fn remove(&self, subdomain: &str) {
        if self.sessions.remove(subdomain).is_some() {
            info!(subdomain = %subdomain, "tunnel removed");
        }
    }

    /// Check if a subdomain is registered.
    #[inline]
    pub fn contains(&self, subdomain: &str) -> bool {
        self.sessions.contains_key(subdomain)
    }

    /// Resolve a subdomain: use the requested name if available, otherwise random.
    /// Evicts stale sessions (dead QUIC connections) holding the requested name.
    pub fn resolve_subdomain(&self, requested: Option<&str>) -> String {
        if let Some(name) = requested.filter(|n| !n.is_empty()) {
            match self.sessions.get(name) {
                None => return name.to_string(),
                Some(existing) if existing.connection.close_reason().is_some() => {
                    drop(existing);
                    self.remove(name);
                    return name.to_string();
                }
                _ => {} // taken by a live session, fall through to random
            }
        }
        self.generate_subdomain()
    }

    /// Generate a unique subdomain like "swift-fox-3847".
    ///
    /// Pattern: adjective-noun-NNNN (4-digit suffix).
    fn generate_subdomain(&self) -> String {
        let mut generator = Generator::default();
        loop {
            let num = rand::random::<u16>() % 10000;
            if let Some(base) = generator.next() {
                let name = format!("{base}-{num:04}");
                if !self.sessions.contains_key(&name) {
                    return name;
                }
            }
        }
    }
}
