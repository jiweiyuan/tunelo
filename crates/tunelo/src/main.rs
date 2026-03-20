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

        /// Maximum tunnel session duration in seconds (0 = no limit).
        #[clap(long, default_value = "7200")]
        max_session: u64,
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
            port, relay, local_host, private, code,
        } => {
            let access_code = resolve_access_code(private, code);
            tunnel::run_tunnel(port, local_host, relay, access_code).await
        }

        Command::Serve {
            path, local, port, relay, private, code,
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
                let display = path.canonicalize().unwrap_or(path.clone());
                let port = fileserver::start_background(path).await?;
                println!("  \x1b[90m▸ Serving {} on :{port}\x1b[0m", display.display());
                tunnel::run_tunnel(port, "127.0.0.1".into(), relay, access_code).await
            }
        }

        Command::Relay {
            domain, tunnel_addr, http_addr, max_session,
        } => {
            tunelo_relay::run(domain, tunnel_addr, http_addr, max_session).await
        }
    }
}

fn resolve_access_code(private: bool, code: Option<String>) -> Option<String> {
    if private { Some(generate_code()) } else { code }
}

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
