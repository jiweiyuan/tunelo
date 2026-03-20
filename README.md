# Tunelo

Expose anything to the internet — local ports, files, directories.

```
$ tunelo http 3000
  ✔ Tunnel is ready!

  Public URL:  https://abc123.tunelo.net
  Forwarding:  → http://localhost:3000
```

```
$ tunelo .
  ▸ Serving /Users/you/project on :51234
  ✔ Tunnel is ready!

  Public URL:  https://xyz789.tunelo.net
  Forwarding:  → file server (web explorer)
```

## Architecture

```
Browser → HTTPS → Gateway → QUIC stream → Client → localhost:3000
                  (8 MB)                   (8 MB)
```

- **QUIC tunnel** (quinn + rustls) — multiplexed, encrypted, low-latency
- **Zero-copy data plane** — `copy_bidirectional` between TCP and QUIC streams
- **Built-in file server** — embedded React frontend with file explorer, viewers for code/markdown/PDF/images/video/audio/CSV/Excel

## Quick Start

```bash
# Build
cargo build --release

# Terminal 1: Start the gateway
./target/release/tunelo-gateway --domain localhost

# Terminal 2: Expose a local service
./target/release/tunelo http 3000 --gateway 127.0.0.1:4433

# Or serve a directory
./target/release/tunelo . --gateway 127.0.0.1:4433

# Or just preview locally (no tunnel)
./target/release/tunelo . --local
```

## CLI

### Client

```
# Port mode — expose a local HTTP service
tunelo http <PORT>                          # Expose HTTP service
tunelo http <PORT> --subdomain myapp        # Request specific subdomain
tunelo http <PORT> --gateway host:4433      # Custom gateway
tunelo http <PORT> -H 0.0.0.0              # Forward to non-localhost
tunelo http <PORT> --private                # Private tunnel (auto access code)
tunelo http <PORT> --code mysecret          # Private tunnel (specific code)

# File mode — serve files with built-in web explorer
tunelo .                                    # Serve current directory
tunelo ./dist                               # Serve a specific directory
tunelo . --subdomain files                  # With custom subdomain
tunelo . --local                            # Local-only preview (no tunnel)
tunelo . -l -p 8000                         # Local preview on port 8000
```

### Gateway

```
tunelo-gateway                              # Start with defaults
tunelo-gateway --domain tunelo.net          # Production domain
tunelo-gateway --tunnel-addr 0.0.0.0:4433   # QUIC listener
tunelo-gateway --http-addr 0.0.0.0:80       # HTTP listener
```

## File Server Features

When you run `tunelo .` or `tunelo ./some-dir`, tunelo starts a built-in file server with:

- **Web Explorer** — browse directories, navigate with breadcrumbs
- **File viewers** — syntax-highlighted code, rendered markdown, PDF viewer, image/video/audio players, CSV/Excel tables
- **Range requests** — streaming support for large files and media seeking
- **Embedded frontend** — the React app is compiled into the binary, zero external dependencies
- **Path traversal protection** — sanitized paths, symlink-safe

The file server exposes two API endpoints:
- `/_api/ls?path=/` — JSON directory listing
- `/_api/raw?path=/file.txt` — raw file content (with Range header support)

Everything else serves the embedded SPA frontend.

## Project Structure

```
crates/
  protocol/     Shared protocol types + codec
  client/       CLI + tunnel client + built-in file server
  gateway/      Gateway server
web/            File explorer frontend (embedded into client binary)
website/        Landing page (tunelo.net)
deploy/         VPS deployment scripts + configs
skills/         AI agent skill (SKILL.md)
```

## Performance

| Metric | Value |
|--------|-------|
| Gateway memory | 8 MB RSS |
| Tunnel overhead | ~14% vs direct |
| Throughput | ~670 req/s (localhost) |
| Binary size | 3.3–3.5 MB |

## License

MIT
