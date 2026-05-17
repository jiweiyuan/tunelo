//! Tunelo — expose anything to the internet.
//!
//!   tunelo port 3000                 expose local port
//!   tunelo port 3000 -- pnpm dev     run command and tunnel it
//!   tunelo serve .                   serve files with web explorer
//!   tunelo relay                     start the relay server

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tokio::net::TcpStream;
use tokio::process::Command as TokioCommand;

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
    /// Expose a local port through a public URL.
    ///
    /// Run a command and tunnel it:
    ///   tunelo port 3000 -- pnpm dev
    ///   tunelo port 5173 -- vite
    Port {
        /// Local port to expose.
        port: u16,

        /// Relay server address.
        #[clap(short, long, env = "TUNELO_RELAY", default_value = "tunelo.net:4433")]
        relay: String,

        /// Local host to forward to.
        #[clap(short = 'H', long, default_value = "localhost")]
        local_host: String,

        /// Protect the tunnel with a password.
        /// Use without a value to auto-generate one, or specify your own.
        #[clap(long, num_args = 0..=1, default_missing_value = "__auto__")]
        password: Option<String>,

        /// Command to run (everything after --).
        #[clap(last = true)]
        command: Vec<String>,
    },

    /// Serve a file or directory with the built-in web explorer.
    Serve {
        /// File or directory to serve (defaults to current directory).
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

        /// Protect the tunnel with a password.
        /// Use without a value to auto-generate one, or specify your own.
        #[clap(long, num_args = 0..=1, default_missing_value = "__auto__")]
        password: Option<String>,
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
        #[clap(long, default_value = "86400")]
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
        Command::Port {
            port, relay, local_host, password, command,
        } => {
            let password = resolve_password(password);

            if !command.is_empty() {
                run_with_command(port, &local_host, relay, password, command).await
            } else {
                tunnel::run_tunnel(port, local_host, relay, password).await
            }
        }

        Command::Serve {
            path, local, port, relay, password,
        } => {
            if !path.exists() {
                bail!("Path '{}' does not exist", path.display());
            }
            let password = resolve_password(password);

            if local {
                let display = path.canonicalize().unwrap_or(path.clone());
                let port = fileserver::start_on_port(path, port).await?;
                println!();
                println!("  Serving {}", display.display());
                println!();
                println!("  http://localhost:{port}");
                println!();
                println!("  Press Ctrl+C to stop.");
                println!();
                tokio::signal::ctrl_c().await?;
                println!("\n  Stopped.");
                Ok(())
            } else {
                let display = path.canonicalize().unwrap_or(path.clone());
                let port = fileserver::start_background(path).await?;
                println!("  Serving {} on :{port}", display.display());
                tunnel::run_tunnel(port, "127.0.0.1".into(), relay, password).await
            }
        }

        Command::Relay {
            domain, tunnel_addr, http_addr, max_session,
        } => {
            tunelo_relay::run(domain, tunnel_addr, http_addr, max_session).await
        }
    }
}

/// Run a child command (e.g. `pnpm dev`), wait for the port, then start the tunnel.
async fn run_with_command(
    port: u16,
    local_host: &str,
    relay: String,
    password: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    let cmd_display = command.join(" ");
    println!("  Starting: {cmd_display}");
    println!("  PORT={port}");
    println!();

    let mut child = TokioCommand::new(&command[0])
        .args(&command[1..])
        .env("PORT", port.to_string())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start '{}': {e}", command[0]))?;

    // Wait for port to accept connections
    println!("  Waiting for port {port}...");
    let host = local_host.to_string();
    let wait_result = wait_for_port(port, &host, Duration::from_secs(60)).await;

    if wait_result.is_err() {
        // Check if child already exited
        if let Ok(Some(status)) = child.try_wait() {
            bail!("Command '{}' exited with {} before port {port} was ready", cmd_display, status);
        }
        bail!("Timeout waiting for port {port} — command '{}' may not be listening", cmd_display);
    }

    println!("  Port {port} is ready.");
    println!();

    // Run tunnel and child concurrently; exit when either stops
    tokio::select! {
        status = child.wait() => {
            let status: std::process::ExitStatus = status?;
            println!("\n  Command exited with {status}");
            std::process::exit(status.code().unwrap_or(1));
        }
        result = tunnel::run_tunnel(port, host, relay, password) => {
            // Tunnel ended (error or shutdown) — kill the child
            let _ = child.kill().await;
            result
        }
    }
}

/// Poll TCP connect until the port accepts a connection.
async fn wait_for_port(port: u16, host: &str, timeout: Duration) -> Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    let addr = format!("{host}:{port}");

    loop {
        if tokio::time::Instant::now() >= deadline {
            bail!("timeout");
        }
        match TcpStream::connect(&addr).await {
            Ok(_) => return Ok(()),
            Err(_) => tokio::time::sleep(Duration::from_millis(300)).await,
        }
    }
}

fn resolve_password(password: Option<String>) -> Option<String> {
    match password {
        Some(ref v) if v == "__auto__" => Some(generate_password()),
        other => other,
    }
}

fn generate_password() -> String {
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
