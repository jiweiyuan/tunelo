//! Tunelo — expose anything to the internet.
//!
//!   tunelo http 3000          expose local HTTP service
//!   tunelo serve .            serve files with web explorer
//!   tunelo relay              start the relay server

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

mod fileserver;
mod proxy;
mod tunnel;

#[derive(Parser, Debug)]
#[clap(
    name = "tunelo",
    about = "Expose anything to the internet.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Expose a local HTTP service through a public URL.
    Http {
        /// Local port to expose.
        port: u16,

        /// Relay server address.
        #[clap(short, long, env = "TUNELO_RELAY", default_value = "tunelo.net:4433")]
        relay: String,

        /// Local host to forward to.
        #[clap(short = 'H', long, default_value = "localhost")]
        local_host: String,

        /// Make tunnel private (auto-generates an access code).
        #[clap(long, conflicts_with = "code")]
        private: bool,

        /// Make tunnel private with a specific access code.
        #[clap(long, conflicts_with = "private")]
        code: Option<String>,

        /// Tunnel time-to-live (e.g. "2h", "30m", "24h"). Default: 2h, max: 24h.
        #[clap(long, default_value = "2h")]
        ttl: String,
    },

    /// Serve files with the built-in web explorer.
    Serve {
        /// Directory to serve (defaults to current directory).
        #[clap(default_value = ".")]
        path: PathBuf,

        /// Local-only mode: serve files without creating a tunnel.
        #[clap(short, long)]
        local: bool,

        /// Port for local-only mode.
        #[clap(short, long, default_value = "3000")]
        port: u16,

        /// Relay server address.
        #[clap(short, long, env = "TUNELO_RELAY", default_value = "tunelo.net:4433")]
        relay: String,

        /// Make tunnel private (auto-generates an access code).
        #[clap(long, conflicts_with = "code")]
        private: bool,

        /// Make tunnel private with a specific access code.
        #[clap(long, conflicts_with = "private")]
        code: Option<String>,

        /// Tunnel time-to-live (e.g. "2h", "30m", "24h"). Default: 2h, max: 24h.
        #[clap(long, default_value = "2h")]
        ttl: String,
    },

    /// Start the relay server.
    Relay {
        /// Domain suffix for tunnel hostnames (e.g., "tunelo.net").
        #[clap(long, env = "TUNELO_DOMAIN", default_value = "localhost")]
        domain: String,

        /// QUIC listener address for tunnel connections from clients.
        #[clap(long, default_value = "0.0.0.0:4433")]
        tunnel_addr: String,

        /// HTTP listener address for public browser connections.
        #[clap(long, default_value = "0.0.0.0:8080")]
        http_addr: String,

        /// Maximum tunnel TTL in seconds. Clients cannot exceed this.
        #[clap(long, default_value = "86400")]
        max_ttl: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tunelo=info,tunelo_relay=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Http {
            port,
            relay,
            local_host,
            private,
            code,
            ttl,
        } => {
            let access_code = resolve_access_code(private, code);
            let ttl_secs = parse_ttl(&ttl)?;
            tunnel::run_tunnel(port, local_host, relay, access_code, ttl_secs).await
        }

        Command::Serve {
            path,
            local,
            port,
            relay,
            private,
            code,
            ttl,
        } => {
            if !path.exists() {
                bail!("Path '{}' does not exist", path.display());
            }

            let access_code = resolve_access_code(private, code);

            if local {
                let display = path.canonicalize().unwrap_or(path.clone());
                let port = fileserver::start_on_port(path, port).await?;
                println!();
                println!("  \x1b[32m✔\x1b[0m Serving \x1b[1m{}\x1b[0m", display.display());
                println!();
                println!("  \x1b[1;36mhttp://localhost:{port}\x1b[0m");
                println!();
                println!("  Press Ctrl+C to stop.");
                println!();
                tokio::signal::ctrl_c().await?;
                println!("\n  Stopped.");
                Ok(())
            } else {
                let ttl_secs = parse_ttl(&ttl)?;
                let display = path.canonicalize().unwrap_or(path.clone());
                let port = fileserver::start_background(path).await?;
                println!("  \x1b[90m▸ Serving {} on :{port}\x1b[0m", display.display());
                tunnel::run_tunnel(port, "127.0.0.1".into(), relay, access_code, ttl_secs).await
            }
        }

        Command::Relay {
            domain,
            tunnel_addr,
            http_addr,
            max_ttl,
        } => {
            tunelo_relay::run(domain, tunnel_addr, http_addr, max_ttl).await
        }
    }
}

/// Resolve access code from --private / --code flags.
fn resolve_access_code(private: bool, code: Option<String>) -> Option<String> {
    if private {
        Some(generate_code())
    } else {
        code
    }
}

/// Parse TTL string like "2h", "30m", "1h30m", "86400" into seconds.
fn parse_ttl(s: &str) -> Result<u64> {
    // Try plain seconds
    if let Ok(secs) = s.parse::<u64>() {
        return Ok(secs);
    }

    let mut total = 0u64;
    let mut num = String::new();
    for c in s.chars() {
        match c {
            '0'..='9' => num.push(c),
            'h' | 'H' => {
                total += num.parse::<u64>().unwrap_or(0) * 3600;
                num.clear();
            }
            'm' | 'M' => {
                total += num.parse::<u64>().unwrap_or(0) * 60;
                num.clear();
            }
            's' | 'S' => {
                total += num.parse::<u64>().unwrap_or(0);
                num.clear();
            }
            _ => bail!("Invalid TTL format: '{}'. Use e.g. '2h', '30m', '1h30m'", s),
        }
    }
    if !num.is_empty() {
        total += num.parse::<u64>().unwrap_or(0);
    }
    if total == 0 {
        bail!("TTL must be > 0");
    }
    Ok(total)
}

/// Generate a short, human-friendly access code like "fox7291".
fn generate_code() -> String {
    const WORDS: &[&str] = &[
        "sun", "moon", "star", "sky", "lake", "fox", "oak", "elm",
        "rain", "snow", "wind", "leaf", "pine", "wolf", "bear", "hawk",
        "reef", "cove", "dawn", "dusk", "peak", "vale", "glen", "bay",
        "jade", "ruby", "onyx", "iron", "silk", "reef", "tide", "wave",
    ];
    let word = WORDS[rand::random::<usize>() % WORDS.len()];
    let num = rand::random::<u16>() % 10000;
    format!("{word}{num:04}")
}
