---
name: tunelo
description: Expose anything to the internet — local ports, files, directories — through a public HTTPS URL using tunelo. Use this when you need to share a local dev server, serve files, demo an app, or give someone temporary access to localhost.
---

## Quick Start

```bash
# Expose a local HTTP service on port 3000
tunelo http 3000

# Serve a directory with built-in web explorer
tunelo serve .

# Output:
#   ✔ Tunnel is ready!
#   Public URL:  https://abc123.tunelo.net
```

## Install

```bash
# Download the binary (Linux amd64)
curl -fsSL https://tunelo.net/install.sh | sh

# Or build from source
cargo install tunelo-client
```

## Commands

### Expose a local service (port mode)

```bash
tunelo http <PORT>                          # Expose HTTP service
tunelo http <PORT> --subdomain myapp        # Request specific subdomain
tunelo http <PORT> --relay tunelo.net:4433  # Custom relay (default: tunelo.net)
tunelo http <PORT> -H 0.0.0.0              # Forward to non-localhost host
tunelo http <PORT> --private                # Private tunnel (auto access code)
tunelo http <PORT> --code mysecret          # Private tunnel (specific code)
```

### Serve files (file mode)

```bash
tunelo serve .                                    # Serve current directory
tunelo serve ./dist                               # Serve a specific directory
tunelo serve . --subdomain files                  # With custom subdomain
tunelo serve . --local                            # Local-only preview (no tunnel)
tunelo serve . -l -p 8000                         # Local preview on port 8000
```

File mode starts a built-in web explorer with directory browsing, code highlighting, markdown rendering, PDF/image/video/audio viewers, and CSV/Excel tables. The frontend is embedded in the binary.

### Common examples

```bash
# Share a React dev server
tunelo http 5173

# Share project files with a colleague
tunelo serve . --subdomain demo

# Preview a static site locally
tunelo serve ./dist --local

# Expose an API with access control
tunelo http 3001 --private
```

## How It Works

```
Browser → HTTPS → Relay → QUIC stream → Client → localhost:PORT
                                                → file server
```

1. Client opens a QUIC tunnel to the relay
2. Relay assigns a public subdomain (`abc123.tunelo.net`)
3. When a browser hits that URL, traffic is relayed through the tunnel to your localhost
4. Zero-copy data plane — no buffering, low overhead (~14% vs direct)

## Tips

- The tunnel stays open as long as the `tunelo` process is running
- Press `Ctrl+C` to close the tunnel
- Subdomains are first-come-first-served; use `--subdomain` to request a specific one
- The public URL is HTTPS — TLS is terminated at the relay
- Use `--private` to generate an access code, or `--code` to set your own
- File mode embeds the web explorer in the binary — no external dependencies
