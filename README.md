# Tunelo

Expose anything to the internet — local ports, files, directories.

```
$ tunelo http 3000
  ✔ Tunnel is ready!

  Public URL:  https://abc123.tunelo.net
  Forwarding:  → http://localhost:3000
```

```
$ tunelo serve .
  ▸ Serving /Users/you/project on :51234
  ✔ Tunnel is ready!

  Public URL:  https://xyz789.tunelo.net
  Forwarding:  → file server (web explorer)
```

## Architecture

```
Browser → HTTPS → Relay → QUIC stream → Client → localhost:3000
                 (8 MB)                  (8 MB)
```

- **QUIC tunnel** (quinn + rustls) — multiplexed, encrypted, low-latency
- **Zero-copy data plane** — `copy_bidirectional` between TCP and QUIC streams
- **Built-in file server** — embedded React web explorer with viewers for code/markdown/PDF/images/video/audio/CSV/Excel
- **Decoupled client + relay** — client defaults to `tunelo.net`, or self-host your own relay
- **One binary** — `tunelo http`, `tunelo serve`, `tunelo relay` — client and server in one

## Quick Start

```bash
# Build
cargo build --release

# Terminal 1: Start a relay (or use the public one at tunelo.net)
./target/release/tunelo relay --domain localhost

# Terminal 2: Expose a local service
./target/release/tunelo http 3000

# Or serve a directory
./target/release/tunelo serve .

# Or just preview locally (no tunnel)
./target/release/tunelo serve . --local
```

## CLI

```
tunelo http <PORT>                          # Expose local HTTP service
tunelo http <PORT> --subdomain myapp        # Request specific subdomain
tunelo http <PORT> --relay host:4433        # Custom relay server
tunelo http <PORT> -H 0.0.0.0              # Forward to non-localhost
tunelo http <PORT> --private                # Private tunnel (auto access code)
tunelo http <PORT> --code mysecret          # Private tunnel (specific code)

tunelo serve .                              # Serve current directory
tunelo serve ./dist                         # Serve a specific directory
tunelo serve . --subdomain files            # With custom subdomain
tunelo serve . --local                      # Local-only preview (no tunnel)
tunelo serve . -l -p 8000                   # Local preview on port 8000

tunelo relay                                # Start relay with defaults
tunelo relay --domain tunelo.net            # Production domain
tunelo relay --tunnel-addr 0.0.0.0:4433     # QUIC listener
tunelo relay --http-addr 0.0.0.0:80         # HTTP listener
```

## File Server

When you run `tunelo serve`, tunelo starts a built-in file server with:

- **Web Explorer** — browse directories, navigate with breadcrumbs
- **File viewers** — syntax-highlighted code, rendered markdown, PDF viewer, image/video/audio players, CSV/Excel tables
- **Range requests** — streaming support for large files and media seeking
- **Embedded frontend** — the React app is compiled into the binary, zero external dependencies

## Docker

```bash
docker run -d -p 8080:8080 -p 4433:4433/udp \
  tunelo/tunelo relay --domain yourdomain.com
```

Or with docker-compose:

```bash
docker compose up -d
```

## Project Structure

```
crates/
  protocol/     Shared protocol types + codec
  relay/        Relay server (lib)
  tunelo/       Main binary (client + relay subcommand)
web/            File explorer frontend (embedded into binary)
website/        Landing page (tunelo.net)
deploy/         VPS deployment scripts + configs
skills/         AI agent skill (SKILL.md)
```

## Performance

| Metric | Value |
|--------|-------|
| Relay memory | 8 MB RSS |
| Tunnel overhead | ~14% vs direct |
| Throughput | ~670 req/s (localhost) |
| Binary size | ~4 MB |

## License

MIT
